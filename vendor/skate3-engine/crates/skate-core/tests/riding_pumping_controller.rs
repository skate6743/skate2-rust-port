//! Independent control-flow checks with supplied geometric measurements.
//! These do not implement or certify the missing native geometry operations.
use skate_core::{
    point_graph::PointGraph,
    riding::pumping::{
        controller::{
            GROUND_PUMPING_TIMESTEP, PumpingGeometry, PumpingSample, calculate, update,
            update_ground,
        },
        settings::{PumpingMode, PumpingSettings},
        state::PumpingState,
    },
};

fn constant(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [value; 8],
    }
}
fn settings() -> PumpingSettings {
    PumpingSettings {
        pump_vs_speed: constant(2.0),
        pump_vs_time: constant(1.0),
        min_crouch_vs_ground_angle: constant(0.25),
        compression_vs_ground_angle: constant(0.5),
        compression_vs_deck_angle: constant(0.75),
        height_change_damping: 1.0,
        minimum_height_change: 0.1,
        maximum_height_change: 0.5,
        ground_compression_scale: -4.0,
        deck_compression_scale: -8.0,
        angular_speed_damping: 1.0,
    }
}
fn mode() -> PumpingMode {
    PumpingMode {
        maximum_absorption_per_second: 100.0,
        maximum_acceleration_per_second: 100.0,
        absorption_factor: 3.0,
        acceleration_factor: 5.0,
    }
}
fn sample() -> PumpingSample {
    PumpingSample {
        position: [4., 5., 6., 9.],
        normal: [0., 1., 0., 7.],
        com_to_deck_world: [1., 2., 3., 0.],
        deck_angle: 0.0,
        intentional_pumping: 1,
    }
}

struct Measurements {
    height: f32,
    angular: f32,
    calls: Vec<&'static str>,
    times: Vec<f32>,
    inclination_input: Option<f32>,
    fail_speed: bool,
}
impl Measurements {
    fn new(height: f32, angular: f32) -> Self {
        Self {
            height,
            angular,
            calls: vec![],
            times: vec![],
            inclination_input: None,
            fail_speed: false,
        }
    }
}
impl PumpingGeometry for Measurements {
    type Error = &'static str;
    fn height(&mut self, normal: [f32; 4], com: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("height");
        assert_eq!(normal, sample().normal);
        assert_eq!(com, sample().com_to_deck_world);
        Ok(self.height)
    }
    fn speed(&mut self, _old: [f32; 4], _now: [f32; 4], dt: f32) -> Result<f32, Self::Error> {
        self.calls.push("speed");
        self.times.push(dt);
        if self.fail_speed {
            Err("native speed unavailable")
        } else {
            Ok(4.0)
        }
    }
    fn angular_speed(
        &mut self,
        _position: [f32; 4],
        _normal: [f32; 4],
        _sample: &PumpingSample,
        dt: f32,
    ) -> Result<f32, Self::Error> {
        self.calls.push("angular");
        self.times.push(dt);
        Ok(self.angular)
    }
    fn inclination_radians(&mut self, input: f32) -> Result<f32, Self::Error> {
        self.calls.push("inclination");
        self.inclination_input = Some(input);
        Ok(0.0)
    }
}
fn seeded() -> PumpingState {
    let mut state = PumpingState::reset_state();
    state.record_valid = true;
    state.previous_normal = [0., 0.5, 0., 0.];
    state
}

#[test]
fn first_update_only_records_history_and_clears_force() {
    let mut state = PumpingState::reset_state();
    state.pump_acceleration = 42.0;
    state.absorption = 3.0;
    state.reset_only_scalar = 11.0;
    state.intentional_pumping = 7;
    let mut geometry = Measurements::new(2.0, 99.0);
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut geometry,
    )
    .unwrap();
    assert_eq!(geometry.calls, ["height"]);
    assert_eq!(state.previous_position, sample().position);
    assert_eq!(state.previous_normal, sample().normal);
    assert_eq!(state.previous_height, 2.0);
    assert!(state.record_valid);
    assert_eq!(state.pump_acceleration, 0.0);
    assert_eq!(state.absorption, 3.0);
    assert_eq!(state.reset_only_scalar, 11.0);
    assert_eq!(state.intentional_pumping, 7);
}

#[test]
fn positive_pumping_caps_height_before_mode_scaling_and_updates_outputs() {
    let mut state = seeded();
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut Measurements::new(2.0, 2.0),
    )
    .unwrap();
    assert_eq!(state.smoothed_height_change, 2.0);
    assert_eq!(state.pumping_time, 0.25);
    assert_eq!(state.pumping, 4.0);
    assert_eq!(state.pump_acceleration, 10.0); // 4 * 5 * capped 0.5
    assert_eq!(state.absorption, -8.0);
    assert_eq!(state.ground_normal_absorption, -6.0); // greater squared deck magnitude
    assert_eq!(state.minimum_crouch, 0.25);
    assert_eq!(state.physics_output().compression, 0.0);
    assert_eq!(state.physics_output().intentional_pumping, 1);
}

#[test]
fn absorption_and_acceleration_use_distinct_factors_and_time_scaled_limits() {
    let mut limits = mode();
    limits.maximum_absorption_per_second = 2.0;
    limits.maximum_acceleration_per_second = 4.0;
    for (angular, expected) in [(-2.0, -0.5), (2.0, 1.0)] {
        let mut state = seeded();
        update(
            &mut state,
            &settings(),
            limits,
            sample(),
            0.25,
            &mut Measurements::new(2.0, angular),
        )
        .unwrap();
        assert_eq!(state.pump_acceleration, expected);
    }
    let mut state = seeded();
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut Measurements::new(2.0, -2.0),
    )
    .unwrap();
    assert_eq!(state.pump_acceleration, -6.0); // -4 * absorption factor 3 * 0.5
}

#[test]
fn falling_height_does_not_pump_but_geometry_and_compression_still_update() {
    let mut state = seeded();
    state.previous_height = 5.0;
    state.pumping_time = 9.0;
    let mut geometry = Measurements::new(2.0, 2.0);
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut geometry,
    )
    .unwrap();
    assert_eq!(state.pumping_time, 0.0);
    assert_eq!(state.pump_acceleration, 0.0);
    assert_eq!(state.absorption, -8.0);
    assert_eq!(
        geometry.calls,
        ["height", "speed", "angular", "inclination"]
    );
    assert_eq!(geometry.inclination_input, Some(0.5)); // previous normal, not this tick's normal
}

#[test]
fn ground_pumping_uses_its_native_fixed_step() {
    let mut state = seeded();
    let mut geometry = Measurements::new(0.25, 2.0);
    update_ground(&mut state, &settings(), mode(), sample(), &mut geometry).unwrap();
    assert_eq!(state.pumping_time.to_bits(), 0x3c88_8889);
    assert_eq!(geometry.times, [GROUND_PUMPING_TIMESTEP; 2]);
}

#[test]
fn reset_invalidates_history_and_all_owned_outputs() {
    let mut state = seeded();
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut Measurements::new(2.0, 2.0),
    )
    .unwrap();
    state.reset_only_scalar = 7.0;
    state.reset();
    assert_eq!(state, PumpingState::reset_state());
    let mut geometry = Measurements::new(2.0, 2.0);
    update_ground(&mut state, &settings(), mode(), sample(), &mut geometry).unwrap();
    assert_eq!(geometry.calls, ["height"]);
    assert_eq!(state.intentional_pumping, 0);
}

#[test]
fn missing_geometry_aborts_before_cache_commit_or_force_publication() {
    let mut state = seeded();
    state.previous_position = [8.0; 4];
    state.pump_acceleration = 12.0;
    let mut geometry = Measurements::new(2.0, 2.0);
    geometry.fail_speed = true;
    assert_eq!(
        update_ground(&mut state, &settings(), mode(), sample(), &mut geometry),
        Err("native speed unavailable")
    );
    assert_eq!(state.previous_position, [8.0; 4]);
    assert_eq!(state.previous_height, 0.0);
    assert_eq!(state.pump_acceleration, 0.0);
    assert_eq!(geometry.calls, ["height", "speed"]);
}

#[test]
fn calculate_preserves_update_owned_history_and_equal_compression_magnitude() {
    let mut state = seeded();
    state.previous_normal[1] = -2.0;
    state.pump_acceleration = 11.0;
    state.pumping_time = 4.0;
    let mut settings = settings();
    settings.deck_compression_scale = 8.0 / 3.0;
    let mut geometry = Measurements::new(1.0, 2.0);
    assert_eq!(
        calculate(&mut state, &settings, &sample(), 0.25, &mut geometry).unwrap(),
        4.0
    );
    assert_eq!(state.ground_normal_absorption, -2.0); // tie does not select positive deck compression
    assert_eq!(geometry.inclination_input, Some(0.0));
    assert_eq!(state.pump_acceleration, 11.0);
    assert_eq!(state.pumping_time, 4.0);
}

#[test]
fn threshold_equality_and_negative_history_keep_native_signed_comparisons() {
    let mut config = settings();
    config.minimum_height_change = 0.25;
    let mut state = seeded();
    update(
        &mut state,
        &config,
        mode(),
        sample(),
        0.25,
        &mut Measurements::new(0.25, 2.0),
    )
    .unwrap();
    assert_eq!(state.pump_acceleration, 5.0); // equality is active, not discarded
    config.height_change_damping = 0.0;
    config.minimum_height_change = -3.0;
    state.smoothed_height_change = -2.0;
    update(
        &mut state,
        &config,
        mode(),
        sample(),
        0.25,
        &mut Measurements::new(0.25, 2.0),
    )
    .unwrap();
    assert_eq!(state.pump_acceleration, -6.0);
    assert_eq!(state.pumping_time, 0.0);
}

#[test]
fn unordered_history_and_normal_follow_native_compare_and_select_paths() {
    let mut state = seeded();
    state.smoothed_height_change = f32::NAN;
    state.previous_normal[1] = f32::NAN;
    let mut geometry = Measurements::new(0.25, 2.0);
    update(
        &mut state,
        &settings(),
        mode(),
        sample(),
        0.25,
        &mut geometry,
    )
    .unwrap();
    assert!(state.smoothed_height_change.is_nan());
    assert_eq!(state.pumping_time, 0.0);
    assert_eq!(state.pump_acceleration, 25.0); // upper fsel selects bound for unordered difference
    assert_eq!(geometry.inclination_input, Some(1.0));
}
