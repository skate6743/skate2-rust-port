use super::*;
use crate::point_graph::PointGraph;

#[derive(Clone, Debug, Default, PartialEq)]
struct FakeLaunchInfo {
    start_velocity: [f32; 4],
    animation_position: [f32; 4],
    trajectory_override: [f32; 4],
    board_override: [f32; 4],
    cone_angles: [f32; 2],
    jumped_writes: Vec<bool>,
    use_override: bool,
    trajectory_count: u16,
}

impl PhysicsAirLaunchInfo for FakeLaunchInfo {
    fn set_start_velocity(&mut self, velocity: [f32; 4]) {
        self.start_velocity = velocity;
    }

    fn centre_of_mass_animation_position(&self) -> [f32; 4] {
        self.animation_position
    }

    fn set_trajectory_start_position_override(&mut self, position: [f32; 4]) {
        self.trajectory_override = position;
    }

    fn set_board_position_override(&mut self, position: [f32; 4]) {
        self.board_override = position;
    }

    fn cone_angles(&self) -> [f32; 2] {
        self.cone_angles
    }

    fn set_cone_angles(&mut self, angles: [f32; 2]) {
        self.cone_angles = angles;
    }

    fn set_player_jumped(&mut self, jumped: bool) {
        self.jumped_writes.push(jumped);
    }

    fn set_use_trajectory_start_position_override(&mut self, enabled: bool) {
        self.use_override = enabled;
    }

    fn set_trajectory_count(&mut self, count: u16) {
        self.trajectory_count = count;
    }
}

#[derive(Debug)]
struct FakeRuntime {
    events: Vec<String>,
    board_height: f32,
    board_velocity: [f32; 4],
    query_started: bool,
    selector_normal: Option<[f32; 4]>,
    selector_com_ready: bool,
    angle: f32,
    collision: bool,
    collision_force: AirBoardForce,
    enqueued_force: Option<(u32, AirBoardForce)>,
    set_velocity: Option<[f32; 4]>,
    launched: Option<FakeLaunchInfo>,
    reckoning_call: Option<([f32; 4], f32, f32, f32)>,
    reckoning_output: PhysicsAirReckoningFields,
    collision_reference: Option<[f32; 4]>,
    known_air_position: Option<[f32; 4]>,
    forced_squared_length: Option<f32>,
    forced_length: Option<f32>,
    forced_clamp: Option<[f32; 4]>,
}

impl Default for FakeRuntime {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            board_height: 0.0,
            board_velocity: [0.0; 4],
            query_started: false,
            selector_normal: None,
            selector_com_ready: false,
            angle: 0.0,
            collision: false,
            collision_force: AirBoardForce::ZERO,
            enqueued_force: None,
            set_velocity: None,
            launched: None,
            reckoning_call: None,
            reckoning_output: reckoning(),
            collision_reference: None,
            known_air_position: None,
            forced_squared_length: None,
            forced_length: None,
            forced_clamp: None,
        }
    }
}

impl PhysicsAirMath for FakeRuntime {
    fn minimum_vminfp(&mut self, left: f32, right: f32) -> f32 {
        if left < right { left } else { right }
    }

    fn length_squared_vmsum3fp(&mut self, value: [f32; 4]) -> f32 {
        self.forced_squared_length
            .unwrap_or(value[0] * value[0] + value[1] * value[1] + value[2] * value[2])
    }

    fn length_vmsum3fp_vrsqrte(&mut self, value: [f32; 4]) -> f32 {
        self.forced_length
            .unwrap_or((value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt())
    }

    fn clamp_vector_within_max_length(&mut self, value: [f32; 4], maximum_length: f32) -> [f32; 4] {
        if let Some(forced) = self.forced_clamp {
            return forced;
        }
        let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
        if length > maximum_length {
            value.map(|lane| lane * (maximum_length / length))
        } else {
            value
        }
    }
}

impl PhysicsAirRuntime for FakeRuntime {
    type LaunchInfo = FakeLaunchInfo;

    fn set_air_collision_update_enabled(&mut self, enabled: bool) {
        self.events.push(format!("collision-enabled:{enabled}"));
    }

    fn set_skeleton_collision_state(&mut self, state: u32) {
        self.events.push(format!("collision-state:{state}"));
    }

    fn set_skeleton_inverse_kinematics_enabled(&mut self, enabled: bool) {
        self.events.push(format!("ik:{enabled}"));
    }

    fn enable_board_angular_drive_only(&mut self) {
        self.events.push("angular-drive".into());
    }

    fn set_footplant_flag_240(&mut self, value: bool) {
        self.events.push(format!("footplant-240:{value}"));
    }

    fn reset_footplants(&mut self) {
        self.events.push("footplant-reset".into());
    }

    fn board_transform_height(&mut self) -> f32 {
        self.events.push("board-height".into());
        self.board_height
    }

    fn request_skeleton_heading_update(&mut self) {
        self.events.push("heading".into());
    }

    fn update_air_collision(&mut self) {
        self.events.push("air-collision".into());
    }

    fn trajectory_query_just_started(&self) -> bool {
        self.query_started
    }

    fn construct_trajectory_launch_info(&mut self) -> Self::LaunchInfo {
        self.events.push("launch-construct".into());
        FakeLaunchInfo::default()
    }

    fn fill_skeleton_launch_info(&mut self, info: &mut Self::LaunchInfo) {
        self.events.push("launch-fill".into());
        info.animation_position = [10.0, 20.0, 30.0, 40.0];
        info.cone_angles = [2.0, 4.0];
    }

    fn launch_trajectory(&mut self, info: &Self::LaunchInfo) {
        self.events.push("launch".into());
        self.launched = Some(info.clone());
    }

    fn update_trajectory_selector(&mut self) {
        self.events.push("selector-update".into());
    }

    fn selector_landing_normal(&self) -> Option<[f32; 4]> {
        self.selector_normal
    }

    fn selector_centre_of_mass_trajectory_ready(&self) -> bool {
        self.selector_com_ready
    }

    fn angle_between_vectors(&mut self, _left: [f32; 4], _right: [f32; 4]) -> f32 {
        self.events.push("angle".into());
        self.angle
    }

    fn update_reckoning_air_states(
        &mut self,
        landing_normal: [f32; 4],
        normal_blend: f32,
        body_spin: f32,
        body_flip: f32,
    ) -> PhysicsAirReckoningFields {
        self.events.push("reckoning".into());
        self.reckoning_call = Some((landing_normal, normal_blend, body_spin, body_flip));
        self.reckoning_output
    }

    fn update_known_air_skeleton(&mut self, trajectory_position: [f32; 4]) {
        self.events.push("known-air".into());
        self.known_air_position = Some(trajectory_position);
    }

    fn update_animated_skateboard_skeleton(&mut self, argument: bool) {
        self.events.push(format!("animated-board:{argument}"));
    }

    fn update_board_steering_tilt(&mut self, tilt: f32) {
        self.events.push(format!("steering-tilt:{tilt}"));
    }

    fn calculate_air_collision_force(
        &mut self,
        reference_normal: [f32; 4],
        force: &mut AirBoardForce,
    ) -> bool {
        self.events.push("collision-force".into());
        self.collision_reference = Some(reference_normal);
        *force = self.collision_force;
        self.collision
    }

    fn enqueue_board_force(&mut self, tag: u32, force: AirBoardForce) {
        self.events.push("enqueue-force".into());
        self.enqueued_force = Some((tag, force));
    }

    fn set_board_velocity(&mut self, velocity: [f32; 4]) {
        self.events.push("set-board-velocity".into());
        self.set_velocity = Some(velocity);
    }

    fn enable_skateboard_error_on_skeleton(&mut self) {
        self.events.push("board-error".into());
    }

    fn board_body_velocity(&mut self) -> [f32; 4] {
        self.events.push("board-velocity".into());
        self.board_velocity
    }

    fn check_for_air_wipeout(&mut self, argument: bool) {
        self.events.push(format!("wipeout:{argument}"));
    }
}

fn frame() -> PhysicsAirFrame {
    PhysicsAirFrame {
        current_velocity_400: [1.0, 2.0, 3.0, 0.0],
        ground_position_y_500: 4.0,
        trajectory_position_592: [1.0, 5.0, 2.0, 0.0],
        trajectory_velocity_608: [2.0, 10.0, 4.0, 0.0],
        jump_velocity_848: [2.0, 8.0, 4.0, 0.0],
        flags_2468: 0,
        previous_physics_state_2504: 500,
        previous_physics_category_2516: 0,
        frames_since_jump_correction_2576: 12,
        delta_time_2604: 0.1,
        body_spin_input_2640: 3.0,
        gravity_y_2648: -10.0,
        state_timer_2664: 1.0,
    }
}

fn state() -> PhysicsAirState {
    PhysicsAirState {
        centre_of_mass_trajectory: AirTrajectory {
            position: [90.0; 4],
            velocity: [80.0; 4],
            acceleration: [70.0; 4],
            scalar_48: 60.0,
        },
        landing_normal: [9.0; 4],
        time_in_state: 9.0,
        start_y: 9.0,
        max_y: 9.0,
        reached_apex: true,
        use_centre_of_mass_velocity: false,
        selector_latch_174: true,
        trajectory_query_countdown: 9,
    }
}

fn reckoning() -> PhysicsAirReckoningFields {
    PhysicsAirReckoningFields {
        current_landing_normal_1152: [0.0, 1.0, 0.0, 0.0],
        collision_reference_normal_1216: [0.0, 0.0, 1.0, 0.0],
        body_spin_angle_1568: 2.0,
        body_spin_speed_1572: 3.0,
    }
}

fn settings() -> PhysicsAirSettings {
    PhysicsAirSettings {
        body_spin_over_time_320: PointGraph {
            x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            y: [2.0; 8],
        },
        landing_normal_blend_388: 0.25,
        body_spin_scale_428: 4.0,
        landing_normal_angle_limit_444: 1.0,
    }
}

#[test]
fn enter_preserves_order_and_offboard_velocity_rule() {
    let mut state = state();
    let frame = frame();
    let mut runtime = FakeRuntime {
        board_height: 6.0,
        ..Default::default()
    };
    enter(&mut state, &frame, [0.0, -9.0, 0.0, 0.0], &mut runtime);

    assert_eq!(
        runtime.events,
        [
            "collision-enabled:true",
            "collision-state:7",
            "ik:true",
            "collision-state:6",
            "angular-drive",
            "footplant-240:false",
            "footplant-reset",
            "board-height",
            "heading",
        ]
    );
    assert!(state.use_centre_of_mass_velocity);
    assert_eq!(
        state.centre_of_mass_trajectory.velocity,
        [2.0, 3.0, 4.0, 0.0]
    );
    assert_eq!(state.landing_normal, [0.0, 1.0, 0.0, 0.0]);
    assert_eq!((state.start_y, state.max_y), (4.0, 6.0));
    assert!(!state.reached_apex && !state.selector_latch_174);
    assert_eq!(state.trajectory_query_countdown, 0);
}

#[test]
fn category_400_can_select_com_without_requesting_heading() {
    let mut state = state();
    let mut frame = frame();
    frame.previous_physics_category_2516 = 400;
    frame.previous_physics_state_2504 = 100;
    let mut runtime = FakeRuntime::default();

    enter(&mut state, &frame, [0.0; 4], &mut runtime);
    assert!(state.use_centre_of_mass_velocity);
    assert!(!runtime.events.iter().any(|event| event == "heading"));
}

#[test]
fn jump_velocity_uses_current_vertical_and_clamped_current_horizontal() {
    let mut frame = frame();
    frame.current_velocity_400 = [10.0, 4.0, 0.0, 7.0];
    frame.jump_velocity_848 = [2.0, 10.0, 0.0, 5.0];
    let mut runtime = FakeRuntime {
        forced_squared_length: Some(64.0),
        forced_length: Some(2.0),
        forced_clamp: Some([2.0, 0.0, 0.0, 99.0]),
        ..Default::default()
    };

    let result = calculate_velocity_from_jump(&frame, 2, &mut runtime);
    assert_eq!(result, [2.0, 4.0, 0.0, 5.0]);
}

#[test]
fn update_runs_com_launch_and_fixed_step_in_native_order() {
    let mut state = state();
    state.use_centre_of_mass_velocity = true;
    state.selector_latch_174 = false;
    state.trajectory_query_countdown = 0;
    state.time_in_state = 0.25;
    state.max_y = 5.0;
    state.centre_of_mass_trajectory = AirTrajectory {
        position: [1.0, 2.0, 3.0, 0.0],
        velocity: [4.0, 5.0, 6.0, 0.0],
        acceleration: [0.0, -6.0, 0.0, 0.0],
        scalar_48: -1.0,
    };
    let mut frame = frame();
    frame.frames_since_jump_correction_2576 = 12;
    let mut runtime = FakeRuntime {
        board_height: 7.0,
        selector_normal: Some([0.0, 0.8, 0.2, 0.0]),
        reckoning_output: PhysicsAirReckoningFields {
            collision_reference_normal_1216: [1.0, 0.0, 0.0, 0.0],
            ..reckoning()
        },
        ..Default::default()
    };

    update(
        &mut state,
        &frame,
        &settings(),
        &mut reckoning(),
        [0.0, -6.0, 0.0, 0.0],
        &mut runtime,
    );

    assert_eq!(
        runtime.events,
        [
            "air-collision",
            "launch-construct",
            "launch-fill",
            "launch-fill",
            "launch",
            "selector-update",
            "angle",
            "reckoning",
            "known-air",
            "steering-tilt:0",
            "collision-force",
            "board-error",
            "board-height",
        ]
    );
    let launch = runtime.launched.unwrap();
    assert_eq!(launch.jumped_writes, [true, false]);
    assert_eq!(launch.trajectory_override, [10.0, 19.35, 30.0, 40.0]);
    assert_eq!(launch.board_override, [10.0, 19.2, 30.0, 40.0]);
    assert_eq!(launch.cone_angles, [0.8, 1.6]);
    assert!(launch.use_override);
    assert_eq!(launch.trajectory_count, 5);
    assert_eq!(state.trajectory_query_countdown, 7);
    assert_eq!(state.max_y, 7.0);
    assert_eq!(state.time_in_state, 0.35);
    assert_eq!(
        runtime.collision_reference,
        Some([1.0, 0.0, 0.0, 0.0]),
        "Board collision used the pre-update reckoning normal"
    );
}

#[test]
fn skateboard_collision_force_wins_over_direct_velocity_correction() {
    let state = state();
    let force = AirBoardForce {
        force_world: [1.0, 2.0, 3.0, 4.0],
        point_board_local: [5.0, 6.0, 7.0, 8.0],
    };
    let mut runtime = FakeRuntime {
        collision: true,
        collision_force: force,
        ..Default::default()
    };
    let mut frame = frame();
    frame.frames_since_jump_correction_2576 = 0;

    update_skateboard(&state, &frame, &reckoning(), &mut runtime);
    assert_eq!(runtime.enqueued_force, Some((15, force)));
    assert_eq!(runtime.set_velocity, None);
}

#[test]
fn skateboard_miss_applies_jump_velocity_only_before_frame_twelve() {
    let mut state = state();
    state.use_centre_of_mass_velocity = false;
    let mut frame = frame();
    frame.frames_since_jump_correction_2576 = 2;
    frame.current_velocity_400 = [2.0, 4.0, 4.0, 0.0];
    frame.jump_velocity_848 = [2.0, 10.0, 4.0, 0.0];
    let mut runtime = FakeRuntime {
        forced_squared_length: Some(0.0),
        ..Default::default()
    };

    update_skateboard(&state, &frame, &reckoning(), &mut runtime);
    assert_eq!(runtime.set_velocity, Some([2.0, 4.0, 4.0, 0.0]));
    assert_eq!(
        runtime.events,
        ["steering-tilt:0", "collision-force", "set-board-velocity"]
    );
}

#[test]
fn post_physics_latches_only_strict_downward_velocity_then_checks_wipeout() {
    let mut state = state();
    state.reached_apex = false;
    let mut runtime = FakeRuntime {
        board_velocity: [0.0, -0.0, 0.0, 0.0],
        ..Default::default()
    };
    update_post_physics(&mut state, &mut runtime);
    assert!(!state.reached_apex);
    runtime.board_velocity[1] = -0.01;
    update_post_physics(&mut state, &mut runtime);
    assert!(state.reached_apex);
    assert_eq!(
        runtime.events,
        [
            "board-velocity",
            "wipeout:false",
            "board-velocity",
            "wipeout:false"
        ]
    );
}

#[test]
fn exit_and_output_publish_only_recovered_fields() {
    let mut state = state();
    state.max_y = 9.0;
    state.start_y = 4.0;
    state.reached_apex = true;
    state.selector_latch_174 = true;
    state.landing_normal = [1.0, 2.0, 3.0, 4.0];
    let mut reckoning = reckoning();

    let output = fill_physics_output(&state);
    assert_eq!(output.jump_height, 5.0);
    assert_eq!(output.scalar_184_write.map(f32::to_bits), Some(0x4080_0000));
    assert_eq!(output.landing_normal, [1.0, 2.0, 3.0, 4.0]);

    exit(&mut state, &mut reckoning);
    assert!(!state.use_centre_of_mass_velocity);
    assert_eq!(reckoning.body_spin_angle_1568, 0.0);
    assert_eq!(reckoning.body_spin_speed_1572, 0.0);
}

#[test]
fn angle_wrap_uses_recovered_turn_constants() {
    assert_eq!(wrap_signed_angle(0.0).to_bits(), 0.0_f32.to_bits());
    assert!(wrap_signed_angle(4.0) < 0.0);
    assert!(wrap_signed_angle(2.0) > 0.0);
}
