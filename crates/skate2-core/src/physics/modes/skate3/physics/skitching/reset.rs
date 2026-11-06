//! Skitch Reset82d887f0 (Skate 3 0x82d47318), with its late live attribute read.
use super::{IDENTITY, MatrixSlot, Skitching, StateConstructorError, ZERO, read_floats};
impl Skitching {
    /// Reset82d887f0 is also called on each Enter. Unlike construction it
    /// preserves the existing attributes, collaborator links, spline and
    /// descriptor, as well as the fields not written by the native Reset.
    pub(crate) fn reset(&mut self) -> Result<(), StateConstructorError> {
        self.grab_spline_last_space = Some(IDENTITY);
        self.grab_spline_current_space = Some(IDENTITY);
        self.grab_spline_future_space = Some(IDENTITY);
        self.grab_spline_forward_step_transform = Some(IDENTITY);
        self.grab_spline_to_physical_hand_space_transform = Some(IDENTITY);
        self.grab_spline_anim_space_to_ik_hand_space_transform = Some(IDENTITY);
        self.grab_spline_aim_space = Some(IDENTITY);
        self.grab_spline_space = Some(MatrixSlot::Future);
        self.grab_spline_ik_hand_space = Some(MatrixSlot::Future);
        self.grab_spline_physical_hand_space = Some(MatrixSlot::Current);
        self.deck_pos = Some(ZERO);
        self.vector_to_deck_pos_on_aim_space = Some(ZERO);
        self.normalized_vector_to_deck_pos_on_aim_space_ground = Some(ZERO);
        self.vector_to_deck_pos_on_hand_space = Some(ZERO);
        self.normalized_vector_to_deck_pos_on_hand_space_ground = Some(ZERO);
        self.vector_to_deck_pos_on_curve_from_base = Some(ZERO);
        self.normalized_vector_to_deck_pos_on_curve_from_base_ground = Some(ZERO);
        self.grab_spline_rh_pos = Some(ZERO);
        self.grab_spline_lh_pos = Some(ZERO);
        self.grab_spline_follow_point_target = Some(ZERO);
        self.grab_spline_velocity = Some(ZERO);
        self.grab_spline_velocity_ground = Some(ZERO);
        self.normalized_vector_to_skitch_target_ground = Some(ZERO);
        self.ground_normal = Some(ZERO);
        self.grab_spline_speed_ground = Some(0.0);
        self.grab_spline_deck_offset = Some(0.0);
        self.grab_spline_deck_offset_speed = Some(0.0);
        self.grab_spline_false_deck_offset_speed_due_to_turning = Some(0.0);
        self.distance_to_deck_pos_on_curve_from_base_ground = Some(0.0);
        self.deck_dist = Some(0.0);
        self.deck_dist_speed = Some(0.0);
        self.deck_dist_on_aim_space = Some(0.0);
        self.grab_spline_follow_point_offset = Some(0.0);
        self.grab_spline_centrifugal_acceleration = Some(0.0);
        self.grab_spline_turn_acceleration = Some(0.0);
        self.grab_spline_grip_acceleration = Some(0.0);
        self.grab_spline_auto_recentering_acceleration = Some(0.0);
        self.grab_spline_follow_point_acceleration = Some(0.0);
        self.grab_spline_desired_follow_point_velocity = Some(0.0);
        self.grab_spline_follow_point_velocity = Some(0.0);
        self.grab_spline_target_offset = Some(0.0);
        self.grab_spline_hand_target_offset = Some(0.0);
        self.grab_spline_hands_dist = Some(0.0);
        self.grab_spline_hands_dist_delta = Some(0.0);
        self.predicted_left_hand_extension = Some(0.0);
        self.predicted_right_hand_extension = Some(0.0);
        self.turn_scalar = Some(0.0);
        self.grab_spline_half_length_mag = Some(0.0);
        self.grab_spline_half_length_mag_with_edge_gap = Some(0.0);
        self.parametric_position = Some(0.0);
        self.deck_angle = Some(0.0);
        self.anim_deck_angle = Some(0.0);
        self.time_in_skitch_state = Some(0.0);
        self.desired_heading_angle = Some(0.0);
        self.contact_speed = Some(0.0);
        self.time_on_edge_hanging_on_for_dear_life = Some(0.0);
        self.time_hands_not_moving = Some(0.0);
        self.left_blend = Some(0.0);
        self.right_blend = Some(0.0);
        self.left_blend_target = Some(0.0);
        self.right_blend_target = Some(0.0);
        self.grip_hand = Some(0);
        self.anim_crouch = Some(0.0);
        self.time_grabbing = Some(0.0);
        self.centrifugal_force_state = Some(0);
        self.follow_point_mode = Some(0);
        self.follow_point_positive_side_clamped_time = Some(0.0);
        self.follow_point_negative_side_clamped_time = Some(0.0);
        self.delay_until_reskitchable = Some(0.0);
        self.exit_count = Some(0);
        self.skitch_state = Some(0);
        self.left_hand_state = Some(1);
        self.right_hand_state = Some(1);
        self.last_left_hand_state = Some(1);
        self.last_right_hand_state = Some(1);
        self.grab_spline_target_offset_locked = Some(false);
        self.grab_spline_target_offset_active = Some(false);
        self.hands_moving = Some(false);
        self.force_exit = Some(false);
        self.push_forward_on_exit = Some(false);
        self.push_back_on_exit = Some(false);
        self.grab_spline_valid = Some(false);
        self.in_air = Some(false);
        self.skitch_steering_active = Some(false);
        // Skate 3 Reset 0x82d47318: +0x330 zero (0x82d47760), +0x3E4 0.0 (0x82d4771c) and the
        // +0x541 0x10/0x08 bits cleared (0x82d4764c).
        self.speed_force = Some(ZERO);
        self.push_target_speed = Some(0.0);
        self.push_latched = Some(false);
        self.braking = Some(false);
        // Reset preserves mApplySideForce and the uninitialized centrifugal sign.
        self.delay_until_hands_off_matters = Some(
            read_floats::<1>(
                self.attribs.as_ref().expect("constructed attributes"),
                0xcc25_7f85_6577_59a1,
            )?[0],
        );
        self.first_eval = Some(true);
        Ok(())
    }
}
