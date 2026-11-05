//! Physical output stores82D3EEE8. Optional fields are conditional writes,
//! not instructions to clear the corresponding shared output on false paths.
use super::{State, math, orientation};
use crate::physics::skeleton_animation_record::AnimationPartTransform;

pub struct Output {
    pub over_599: bool,
    pub scalar_544: f32,
    pub no_support_time_548: f32,
    pub can_leave_72: bool,
    pub special_surface_81: bool,
    pub below_surface_82: bool,
    pub surface_height_32: f32,
    pub surface_height_valid_83: bool,
    pub collision_time_144: f32,
    pub profile_148: u32,
    pub response_strength_580: f32,
    pub extra_weight_584: f32,
    pub time_until_teleport_576: f32,
    pub teleport_pending_604: bool,
    pub material_ten_3479: bool,
    pub material_eleven_3480: bool,
    pub imminent_surface_twelve_3483: bool,
    pub predicted_position_64: [f32; 4],
    pub retained_air_velocity_160: Option<[f32; 4]>,
    pub response_change_588: Option<f32>,
    pub teleport_countdown_68: bool,
    pub request_teleport_69: bool,
    pub hips_right_angle_496: f32,
    pub hips_up_angle_500: f32,
}
impl State {
    pub fn output(&self, hips: &AnimationPartTransform, normal: [f32; 4]) -> Output {
        let right = hips[0];
        let projected = math::sub(normal, math::scale(right, math::dot(normal, right)));
        let right_angle = orientation::signed_angle(right, normal, [0.0; 4]);
        let up_angle = math::wrap_angle(
            orientation::signed_angle(projected, hips[1], right) + f32::from_bits(0x4049_0FDB),
        );
        Output {
            over_599: self.over,
            scalar_544: self.orientation,
            no_support_time_548: self.no_support_time,
            can_leave_72: self.recovery_eligible && !self.prevent_manual && !self.ignore_reset,
            special_surface_81: self.special_surface,
            below_surface_82: self.below_surface,
            surface_height_32: self.surface_height,
            surface_height_valid_83: true,
            collision_time_144: self.predicted_time,
            profile_148: self.profile as u32,
            response_strength_580: self.response_scalar,
            extra_weight_584: self.extra_weight,
            time_until_teleport_576: self.time_until_teleport,
            teleport_pending_604: self.teleport_pending,
            material_ten_3479: self.material_ten_response,
            material_eleven_3480: self.material_eleven_response,
            imminent_surface_twelve_3483: self.imminent_surface_twelve,
            predicted_position_64: self.predicted_position,
            retained_air_velocity_160: self
                .retained_velocity_active
                .then_some(self.retained_velocity),
            response_change_588: self.response_finished.then_some(self.response_change),
            teleport_countdown_68: self.teleport_countdown >= 0,
            request_teleport_69: self.request_teleport,
            hips_right_angle_496: right_angle,
            hips_up_angle_500: up_angle,
        }
    }
}
