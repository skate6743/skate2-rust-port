//! `PhysState_PhysicsGround::FillPhysOut` (`82D3A388`).
//!
//! The TU3 body is a leaf: it reads Ground and ProcessedPhysIn fields and
//! writes distinct PhysOut records. Names recovered from Skate 2 are used only
//! where the TU3 offsets and operations agree. Remaining fields retain their
//! destination offsets. Xenon vector arithmetic uses the project's isolated
//! accepted host approximation; no generated runtime code is included.

use crate::physics::native_arithmetic;

use super::data::PhysicsGroundState;

pub type Vector4 = [f32; 4];

/// ProcessedPhysIn values consumed by the TU3 output leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOutputFrame {
    pub axis_464: Vector4,
    pub velocity_608: Vector4,
    pub absolute_body_speed_2616: f32,
    pub deck_speed_2652: f32,
    pub state_timer_2664: f32,
    pub scalar_2720: f32,
    pub flags_2476: u32,
    pub flags_2484: u32,
    /// ProcessedPhysIn+2548 -> selected collection+4 -> byte+109.
    pub selected_mode_flag_109: bool,
}

/// Live collection values loaded through the global collection singleton.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundOutputSettings {
    /// Selected ground-layout +4 and +8, added before the pushable-speed test.
    pub pushable_speed_terms_4_8: [f32; 2],
    /// Selected ground-layout +0, used by the conditional output+52/+58 test.
    pub mode_speed_threshold_0: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkateboardMotionOutput {
    /// PhysOut pointer+4, byte+274.
    pub is_push_accelerating: bool,
    /// PhysOut pointer+4, byte+276.
    pub is_at_pushable_speed: bool,
}

/// Conditional writes to the record reached through PhysOut pointer+36.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundVelocityProjectionOutput {
    /// Destination +144.
    pub velocity_without_axis_component: Vector4,
    /// Destination +164. The native branch only ever writes true.
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundRecordOutput {
    /// PhysOut pointer+32, bytes +316/+317/+318.
    pub wall_ride_exit: bool,
    pub anti_flip_nudge_present: bool,
    pub is_pinning: bool,
    /// PhysOut pointer+32, vector+128.
    pub anti_flip_torque: Vector4,
    /// PhysOut pointer+32, floats +276/+280.
    pub time_to_skitch: f32,
    pub skitch_spline_height: f32,
    /// PhysOut pointer+32, float+304.
    pub processed_scalar_2720: f32,
    /// PhysOut pointer+32, byte+325.
    pub processed_flag_2484_bit_13: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateRecordOutput {
    /// PhysOut pointer+28, words +36/+40.
    pub grab_spline_type: u32,
    pub grab_spline_object_id: u32,
    /// PhysOut pointer+28, byte+84. Stock name is not yet proven.
    pub flag_84: bool,
    /// PhysOut pointer+28, byte+86.
    pub has_world_grab_intent_without_object: bool,
    /// PhysOut pointer+28, byte+78 is untouched when this is `None`.
    pub manual_correction_write_78: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntentRecordOutput {
    /// PhysOut pointer+52, byte+55.
    pub has_world_grab_intent: bool,
    /// PhysOut pointer+52, byte+58. Stock name is not yet proven.
    pub selected_mode_below_speed_threshold_58: bool,
}

/// All writes made by TU3 `82D3A388`. Field grouping follows the pointer table
/// reached through the PhysOut argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsGroundOutput {
    pub skateboard_motion_4: SkateboardMotionOutput,
    /// `None` means the native `ProcessedPhysIn+2664 == 0.0` branch did not
    /// touch pointer+36 vector+144 or byte+164.
    pub velocity_projection_36: Option<GroundVelocityProjectionOutput>,
    pub ground_32: GroundRecordOutput,
    pub state_28: StateRecordOutput,
    pub intents_52: IntentRecordOutput,
    /// PhysOut pointer+72, byte+304.
    pub is_grabbing_object_72_304: bool,
    /// PhysOut pointer+56, byte+168.
    pub manual_opposition_56_168: bool,
    /// PhysOut pointer+20, byte+596.
    pub push_suppressed_20_596: bool,
}

/// Complete TU3 output leaf. Every comparison uses the same strict relation as
/// its `fcmpu`/branch pair, so unordered values do not select either true path.
pub fn fill_physics_output(
    state: &PhysicsGroundState,
    frame: GroundOutputFrame,
    settings: GroundOutputSettings,
) -> PhysicsGroundOutput {
    let has_world_grab_intent = frame.flags_2476 & 0x0040_0000 != 0;
    let velocity_projection_36 =
        (frame.state_timer_2664 == 0.0).then(|| GroundVelocityProjectionOutput {
            velocity_without_axis_component: reject_axis_component(
                frame.velocity_608,
                frame.axis_464,
            ),
            active: true,
        });

    PhysicsGroundOutput {
        skateboard_motion_4: SkateboardMotionOutput {
            is_push_accelerating: state.flag_2721,
            is_at_pushable_speed: frame.absolute_body_speed_2616
                < settings.pushable_speed_terms_4_8[0] + settings.pushable_speed_terms_4_8[1],
        },
        velocity_projection_36,
        ground_32: GroundRecordOutput {
            wall_ride_exit: state.flag_2720,
            anti_flip_nudge_present: state.anti_flip_nudge_applied_2723,
            is_pinning: state.pinning_2727,
            anti_flip_torque: state.anti_flip_torque_2624,
            time_to_skitch: state.scalar_2664,
            skitch_spline_height: state.scalar_2668,
            processed_scalar_2720: frame.scalar_2720,
            processed_flag_2484_bit_13: frame.flags_2484 & 0x0000_2000 != 0,
        },
        state_28: StateRecordOutput {
            grab_spline_type: state.word_2560,
            grab_spline_object_id: state.word_2564,
            flag_84: state.flag_2731,
            has_world_grab_intent_without_object: has_world_grab_intent && !state.flag_2729,
            manual_correction_write_78: state.manual_correction_2732.then_some(true),
        },
        intents_52: IntentRecordOutput {
            has_world_grab_intent,
            selected_mode_below_speed_threshold_58: frame.selected_mode_flag_109
                && frame.deck_speed_2652 < settings.mode_speed_threshold_0,
        },
        is_grabbing_object_72_304: state.flag_2729,
        manual_opposition_56_168: state.manual_opposition_2733,
        push_suppressed_20_596: state.push_suppressed_2730,
    }
}

fn reject_axis_component(velocity: Vector4, axis: Vector4) -> Vector4 {
    // Direct TU3 sequence: vmsum3fp128, vmulfp128, vsubfp128. `dot3` is the
    // isolated, explicitly documented host approximation for the Xenon sum.
    let projection = native_arithmetic::dot3(velocity, axis);
    core::array::from_fn(|lane| {
        let scaled = flush_subnormal(axis[lane] * projection);
        flush_subnormal(velocity[lane] - scaled)
    })
}

fn flush_subnormal(value: f32) -> f32 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 == 0 && bits & 0x007f_ffff != 0 {
        f32::from_bits(bits & 0x8000_0000)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PhysicsGroundState {
        PhysicsGroundState {
            collision_force_2528: [0.0; 4],
            collision_point_2544: [0.0; 4],
            word_2560: 0x1122_3344,
            word_2564: 0x5566_7788,
            vector_2592: [0.0; 4],
            vector_2608: [0.0; 4],
            anti_flip_torque_2624: [1.0, 2.0, 3.0, 4.0],
            steering_push_scalar_2640: 1.0,
            steering_damped_turn_2644: 0.0,
            elapsed_2648: 0.0,
            collision_countdown_2652: 0.0,
            captured_position_x_2656: 0.0,
            captured_position_z_2660: 0.0,
            scalar_2664: 6.0,
            scalar_2668: 7.0,
            straighten_scale_2672: 1.0,
            vector_2688: [0.0; 4],
            scalar_2704: 0.0,
            flag_2708: false,
            flag_2720: true,
            flag_2721: true,
            flag_2722: false,
            anti_flip_nudge_applied_2723: true,
            human_player_2724: true,
            controls_latched_2725: false,
            captured_position_valid_2726: true,
            pinning_2727: true,
            was_pinning_2728: false,
            flag_2729: false,
            push_suppressed_2730: true,
            flag_2731: true,
            manual_correction_2732: true,
            manual_opposition_2733: true,
            hang_detection_frames_2740: 0,
            hang_force_frames_2744: 0,
            hung_wipeout_frames_2748: 0,
            anti_flip_nudge_frames_2752: 0,
        }
    }

    fn frame() -> GroundOutputFrame {
        GroundOutputFrame {
            axis_464: [1.0, 0.0, 0.0, 0.0],
            velocity_608: [5.0, 2.0, 3.0, 4.0],
            absolute_body_speed_2616: 4.0,
            deck_speed_2652: 2.0,
            state_timer_2664: 0.0,
            scalar_2720: 8.0,
            flags_2476: 0x0040_0000,
            flags_2484: 0x0000_2000,
            selected_mode_flag_109: true,
        }
    }

    fn settings() -> GroundOutputSettings {
        GroundOutputSettings {
            pushable_speed_terms_4_8: [2.0, 3.0],
            mode_speed_threshold_0: 3.0,
        }
    }

    #[test]
    fn fill_maps_every_unconditional_field_and_projection_branch() {
        let output = fill_physics_output(&state(), frame(), settings());

        assert_eq!(
            output.skateboard_motion_4,
            SkateboardMotionOutput {
                is_push_accelerating: true,
                is_at_pushable_speed: true,
            }
        );
        assert_eq!(
            output.velocity_projection_36,
            Some(GroundVelocityProjectionOutput {
                velocity_without_axis_component: [0.0, 2.0, 3.0, 4.0],
                active: true,
            })
        );
        assert!(output.ground_32.wall_ride_exit);
        assert!(output.ground_32.anti_flip_nudge_present);
        assert!(output.ground_32.is_pinning);
        assert_eq!(output.ground_32.anti_flip_torque, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(output.ground_32.time_to_skitch, 6.0);
        assert_eq!(output.ground_32.skitch_spline_height, 7.0);
        assert_eq!(output.ground_32.processed_scalar_2720, 8.0);
        assert!(output.ground_32.processed_flag_2484_bit_13);
        assert_eq!(output.state_28.grab_spline_type, 0x1122_3344);
        assert_eq!(output.state_28.grab_spline_object_id, 0x5566_7788);
        assert!(output.state_28.flag_84);
        assert!(output.state_28.has_world_grab_intent_without_object);
        assert_eq!(output.state_28.manual_correction_write_78, Some(true));
        assert!(output.intents_52.has_world_grab_intent);
        assert!(output.intents_52.selected_mode_below_speed_threshold_58);
        assert!(!output.is_grabbing_object_72_304);
        assert!(output.manual_opposition_56_168);
        assert!(output.push_suppressed_20_596);
    }

    #[test]
    fn fill_preserves_native_conditional_writes_and_unordered_comparisons() {
        let mut state = state();
        state.flag_2729 = true;
        state.manual_correction_2732 = false;
        let mut frame = frame();
        frame.state_timer_2664 = f32::NAN;
        frame.absolute_body_speed_2616 = f32::NAN;
        frame.deck_speed_2652 = f32::NAN;

        let output = fill_physics_output(&state, frame, settings());

        assert_eq!(output.velocity_projection_36, None);
        assert!(!output.skateboard_motion_4.is_at_pushable_speed);
        assert!(!output.intents_52.selected_mode_below_speed_threshold_58);
        assert!(!output.state_28.has_world_grab_intent_without_object);
        assert_eq!(output.state_28.manual_correction_write_78, None);
    }

    #[test]
    fn fill_extracts_only_the_two_recovered_processed_bits() {
        let mut frame = frame();
        frame.flags_2476 = 0x0020_0000;
        frame.flags_2484 = 0x0000_1000;
        frame.state_timer_2664 = -0.0;

        let output = fill_physics_output(&state(), frame, settings());

        assert!(!output.intents_52.has_world_grab_intent);
        assert!(!output.ground_32.processed_flag_2484_bit_13);
        assert!(output.velocity_projection_36.is_some());
    }
}
