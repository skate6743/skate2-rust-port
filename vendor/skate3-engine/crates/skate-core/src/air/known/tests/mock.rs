use crate::point_graph::PointGraph;

use super::super::*;

pub fn flat_graph(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y: [value; 8],
    }
}

pub fn trajectory() -> KnownAirTrajectory {
    KnownAirTrajectory {
        position: [0.0; 4],
        velocity: [0.0; 4],
        acceleration: [0.0; 4],
        scalar_48: 1.0,
        word_52: 0,
        word_56: 0,
        word_60: 0,
    }
}

pub fn state() -> KnownAirState {
    KnownAirState {
        landing_normal_64: [0.0, 1.0, 0.0, 0.0],
        landing_heading_80: [1.0, 0.0, 0.0, 0.0],
        trajectory_apex_96: [0.0; 4],
        collision_position_112: [0.0; 4],
        selector_vector_128: [0.0; 4],
        trajectory_follow_offset_144: [0.0; 4],
        target_com_position_160: [0.0; 4],
        collision_normal_speed_176: 0.0,
        time_in_state_180: 0.0,
        start_y_184: 0.0,
        max_y_188: 0.0,
        com_max_y_192: 0.0,
        collision_time_196: 1.0,
        time_to_apex_200: 0.0,
        body_flip_target_speed_204: 0.0,
        reached_apex_208: false,
        landing_heading_valid_209: false,
        start_flipped_210: false,
        body_flipping_211: false,
        grind_air_adjust_activated_212: false,
        targeting_grind_213: false,
        trajectory_index_216: 0,
    }
}

pub fn frame() -> KnownAirFrame {
    KnownAirFrame {
        start_flip_reference_96: [0.0; 4],
        alternate_head_target_112: [0.0; 4],
        velocity_400: [0.0; 4],
        ground_normal_464: [0.0, 1.0, 0.0, 0.0],
        start_height_484: 0.0,
        skater_up_544: [0.0, 1.0, 0.0, 0.0],
        flags_2468: 0,
        flags_2472: 0,
        flags_2476: 0,
        flags_2480: 0,
        flags_2484: 0,
        flags_2488: 0,
        next_physics_state_2500: 0,
        delta_time_2604: 1.0 / 60.0,
        forward_speed_2612: 0.0,
        body_spin_input_2640: 0.0,
        selector_landing_normal_2656: [0.0, 1.0, 0.0, 0.0],
    }
}

pub fn settings() -> KnownAirSettings {
    KnownAirSettings {
        max_heading_adjust_vs_up_y_160: flat_graph(180.0),
        landing_speed_scalar_vs_ground_normal_y_240: flat_graph(1.0),
        flip_start_collision_time_vs_normal_y: flat_graph(10.0),
        trajectory_error_blend_away_time_384: 1.0,
        min_target_heading_velocity_420: 0.1,
        min_auto_body_speed_424: 0.01,
        max_spin_speed_428: 360.0,
        frames_for_grind_air_assist_436: 4.0,
        body_flip_min_grab_time_fraction_456: 0.25,
        flip_scalar: 1.0,
    }
}

pub fn mode() -> KnownAirModeSettings {
    KnownAirModeSettings {
        upside_down_falling_wipeout_enabled_2: true,
        perfect_body_flips_28: false,
        body_spin_speed_limit_56: 20.0,
    }
}

pub fn reckoning() -> KnownAirReckoningFields {
    KnownAirReckoningFields {
        landing_normal_1152: [0.0, 1.0, 0.0, 0.0],
        heading_axis_1200: [1.0, 0.0, 0.0, 0.0],
        body_spin_angle_1568: 0.0,
        body_spin_speed_1572: 0.0,
    }
}

pub fn output() -> KnownAirOutput {
    KnownAirOutput {
        trajectory_apex_0: [0.0; 4],
        collision_position_16: [0.0; 4],
        landing_normal_32: [0.0; 4],
        selector_vector_48: [0.0; 4],
        trajectory_position_64: [0.0; 4],
        landing_heading_80: [0.0; 4],
        selector_com_position_96: [0.0; 4],
        landing_normal_copy_144: [0.0; 4],
        locked_trajectory_velocity_160: [0.0; 4],
        time_in_state_176: 0.0,
        collision_time_180: 0.0,
        time_until_collision_184: 0.0,
        collision_normal_speed_188: 0.0,
        time_to_apex_196: 0.0,
        jump_height_200: 0.0,
        trajectory_index_220: 0,
        selected_trajectory_240: trajectory(),
        trajectory_plane_samples_336: [0.0; 25],
        reached_apex_436: false,
        known_air_valid_437: false,
        locked_trajectory_velocity_valid_452: false,
    }
}

pub struct MockRuntime {
    pub calls: Vec<String>,
    pub prediction: KnownAirPrediction,
    pub selector_trajectory: KnownAirTrajectory,
    pub contact: Vector4,
    pub selector_vector: Vector4,
    pub selector_com: Vector4,
    pub highest: (Vector4, f32),
    pub board_y: f32,
    pub com: Vector4,
    pub targeting_grind: bool,
    pub closest: (i32, Vector4),
    pub grind_started: bool,
    pub grind_adjusting: bool,
    pub board_velocity: Vector4,
    pub written_board_velocity: Vector4,
    pub forward: Vector4,
    pub signed_angle: f32,
    pub restore_geometry: RestoreVelocityGeometry,
}

impl MockRuntime {
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            prediction: KnownAirPrediction {
                trajectory: trajectory(),
                collision_time_48: 1.0,
                collision_frame_128: 2,
            },
            selector_trajectory: trajectory(),
            contact: [0.0; 4],
            selector_vector: [0.0; 4],
            selector_com: [0.0; 4],
            highest: ([0.0; 4], 0.0),
            board_y: 0.0,
            com: [0.0; 4],
            targeting_grind: false,
            closest: (0, [0.0; 4]),
            grind_started: false,
            grind_adjusting: false,
            board_velocity: [0.0; 4],
            written_board_velocity: [0.0; 4],
            forward: [1.0, 0.0, 0.0, 0.0],
            signed_angle: 0.0,
            restore_geometry: RestoreVelocityGeometry {
                tangential_velocity: [0.0; 4],
                landing_speed_curve_input: 0.0,
                landing_speed_blend_source: 0.0,
            },
        }
    }

    fn call(&mut self, value: impl Into<String>) {
        self.calls.push(value.into());
    }
}

impl KnownAirMath for MockRuntime {
    fn dot3(&mut self, left: Vector4, right: Vector4) -> f32 {
        left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
    }

    fn divide_vector_by_scalar(&mut self, vector: Vector4, scalar: f32) -> Vector4 {
        vector.map(|value| value / scalar)
    }

    fn normalize_safe_with_length(&mut self, vector: Vector4) -> (Vector4, f32) {
        let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
        if length == 0.0 {
            ([0.0; 4], 0.0)
        } else {
            (vector.map(|value| value / length), length)
        }
    }

    fn signed_angle_about_axis(&mut self, _: Vector4, _: Vector4, _: Vector4) -> f32 {
        self.call("math.signed_angle");
        self.signed_angle
    }

    fn wrap_angle_8258db98(&mut self, angle: f32) -> f32 {
        self.call("math.wrap");
        angle
    }

    fn restore_velocity_geometry_82d35998(
        &mut self,
        _: Vector4,
        _: Vector4,
    ) -> RestoreVelocityGeometry {
        self.call("math.restore_geometry");
        self.restore_geometry
    }

    fn transform_point_82d36880(&mut self, _: AffineTransform, point: Vector4) -> Vector4 {
        point
    }
}

impl KnownAirRuntime for MockRuntime {
    fn enable_board_angular_drive_only(&mut self) {
        self.call("board.angular_only");
    }
    fn set_air_collision_update_enabled(&mut self, value: bool) {
        self.call(format!("collision.enabled:{value}"));
    }
    fn set_skeleton_collision_state(&mut self, value: u32) {
        self.call(format!("collision.state:{value}"));
    }
    fn set_skeleton_physics_truck_tilt_enabled(&mut self, value: bool) {
        self.call(format!("skeleton.truck_tilt:{value}"));
    }
    fn set_skeleton_inverse_kinematics_enabled(&mut self, value: bool) {
        self.call(format!("skeleton.ik:{value}"));
    }
    fn reset_reckoning_flipping(&mut self) {
        self.call("reckoning.reset_flip");
    }
    fn set_footplant_flag_240(&mut self, value: bool) {
        self.call(format!("footplant.flag:{value}"));
    }
    fn reset_footplants(&mut self) {
        self.call("footplant.reset");
    }
    fn board_transform_height(&mut self) -> f32 {
        self.call("board.height");
        self.board_y
    }
    fn skeleton_com_position(&mut self) -> Vector4 {
        self.call("skeleton.com");
        self.com
    }
    fn selector_targeting_grind(&self) -> bool {
        self.targeting_grind
    }
    fn start_grind_air_adjust_from_selector(&mut self) {
        self.call("grind.start");
    }
    fn selector_closest_trajectory_point(&mut self, _: Vector4, _: Vector4) -> (i32, Vector4) {
        self.call("selector.closest");
        self.closest
    }
    fn selected_prediction(&self) -> KnownAirPrediction {
        self.prediction
    }
    fn selector_trajectory_2704(&self) -> KnownAirTrajectory {
        self.selector_trajectory
    }
    fn selector_contact_position(&mut self) -> Vector4 {
        self.call("selector.contact");
        self.contact
    }
    fn selector_vector_2848(&self) -> Vector4 {
        self.selector_vector
    }
    fn selector_com_position_2784(&self) -> Vector4 {
        self.selector_com
    }
    fn highest_trajectory_position(&mut self, _: &KnownAirTrajectory) -> (Vector4, f32) {
        self.call("trajectory.highest");
        self.highest
    }
    fn selector_has_just_changed(&self) -> bool {
        false
    }
    fn reset_trajectory_selector(&mut self) {
        self.call("selector.reset");
    }
    fn begin_reckoning_body_flip(&mut self, side: bool) {
        self.call(format!("reckoning.begin_flip:{side}"));
    }
    fn update_air_collision(&mut self) {
        self.call("collision.update");
    }
    fn grind_air_adjust_started(&self) -> bool {
        self.grind_started
    }
    fn set_grind_air_adjust_started(&mut self, value: bool) {
        self.call(format!("grind.started:{value}"));
    }
    fn set_grind_air_adjust_activated(&mut self, value: bool) {
        self.call(format!("grind.activated:{value}"));
    }
    fn grind_air_adjust_adjusting(&self) -> bool {
        self.grind_adjusting
    }
    fn disable_grind_foot_collision(&mut self, body: GrindFootBody) {
        let name = match body {
            GrindFootBody::Body3152 => "3152",
            GrindFootBody::Body3156 => "3156",
            GrindFootBody::Body3160 => "3160",
            GrindFootBody::Body3168 => "3168",
            GrindFootBody::Body3172 => "3172",
            GrindFootBody::Body3176 => "3176",
        };
        self.call(format!("grind.disable:{name}"));
    }
    fn update_reckoning_air_states(&mut self, _: Vector4, _: f32, _: f32, _: f32) {
        self.call("reckoning.update_air");
    }
    fn update_known_air_skeleton(&mut self, _: Vector4) {
        self.call("skeleton.update_known_air");
    }
    fn set_skeleton_add_skateboard_error(&mut self, value: bool) {
        self.call(format!("skeleton.board_error:{value}"));
    }
    fn set_head_tracking_target(&mut self, _: Vector4, valid: bool) {
        self.call(format!("head.target:{valid}"));
    }
    fn update_board_steering_tilt(&mut self, _: f32) {
        self.call("board.steering_zero");
    }
    fn shift_footplant_trajectory_to_index(&mut self, _: &mut KnownAirTrajectory, _: i32) {
        self.call("footplant.shift");
    }
    fn update_footplant_prediction(&mut self, _: &KnownAirFootplantInput) {
        self.call("footplant.prediction");
    }
    fn update_footplant_lock(&mut self, _: &KnownAirFootplantInput) {
        self.call("footplant.lock");
    }
    fn update_footplant_pose(&mut self, _: &KnownAirFootplantInput) {
        self.call("footplant.pose");
    }
    fn board_body_velocity(&mut self) -> Vector4 {
        self.call("board.velocity");
        self.board_velocity
    }
    fn set_board_velocity(&mut self, value: Vector4) {
        self.call("board.set_velocity");
        self.written_board_velocity = value;
    }
    fn board_forward_axis(&mut self) -> Vector4 {
        self.call("board.forward");
        self.forward
    }
    fn check_for_air_wipeout(&mut self, value: bool) {
        self.call(format!("wipeout.check:{value}"));
    }
    fn reckoning_com_transform_816(&self) -> AffineTransform {
        AffineTransform {
            vectors: [[0.0; 4]; 4],
        }
    }
}
