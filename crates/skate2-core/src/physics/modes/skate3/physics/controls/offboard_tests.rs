//! Numerical and adapter regressions, not original-executable parity tests.
use super::{ActionMap, PlayerControls, SimulationActions, camera_relative_axes};

fn close(actual: [f32; 2], expected: [f32; 2]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!(
            (a - e).abs() < 2.0e-6,
            "actual={actual:?}, expected={expected:?}"
        );
    }
}

#[test]
fn offboard_remap_preserves_native_left_axis_sign_and_yaw() {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    close(camera_relative_axes([0.25, 0.75], identity), [-0.25, 0.75]);
    let quarter_turn = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
    close(
        camera_relative_axes([0.25, 0.75], quarter_turn),
        [0.75, 0.25],
    );
    close(
        camera_relative_axes([-0.25, -0.75], quarter_turn),
        [-0.75, -0.25],
    );
    close(camera_relative_axes([0.0, 0.0], quarter_turn), [0.0, 0.0]);
}

#[test]
fn offboard_remap_flattens_pitch_below_native_threshold() {
    let pitched = [[1.0, 0.0, 0.0], [0.0, 0.8, -0.6], [0.0, 0.6, 0.8]];
    close(camera_relative_axes([0.25, 0.75], pitched), [-0.25, 0.75]);
}

#[test]
fn offboard_remap_retains_camera_basis_at_both_threshold_poles() {
    let threshold = f32::from_bits(0x3f7d_70a4);
    let horizontal = (1.0 - threshold * threshold).sqrt();
    for vertical in [threshold, -threshold] {
        let pitched = [
            [1.0, 0.0, 0.0],
            [0.0, horizontal, -vertical],
            [0.0, vertical, horizontal],
        ];
        close(
            camera_relative_axes([0.25, 0.75], pitched),
            [-0.25, 0.75 * horizontal],
        );
    }
    //An exactly vertical camera does not invent a horizontal forward direction.
    let vertical = [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]];
    close(camera_relative_axes([0.0, 1.0], vertical), [0.0, 0.0]);
}

struct Packet {
    values: [f32; 82],
    states: [u8; 82],
}

impl Packet {
    fn distinct() -> Self {
        Self {
            values: std::array::from_fn(|i| i as f32 / 100.0),
            states: std::array::from_fn(|i| (i % 2) as u8),
        }
    }
}

impl ActionMap for Packet {
    fn value(&mut self, action: u32) -> f32 {
        self.values[action as usize]
    }
    fn state(&mut self, action: u32) -> u8 {
        self.states[action as usize]
    }
}

#[test]
fn offboard_simulation_adapter_overrides_only_the_sampled_pair() {
    let mut packet = Packet::distinct();
    let values = packet.values;
    let states = packet.states;
    let mut controls = PlayerControls::default();
    controls.offboard_axes = Some([-0.25, 0.0]);
    let mut mapped = controls.simulation_actions(&mut packet);
    //The adapter owns the sampled pair, not a borrow of mutable controller state.
    controls.offboard_axes = Some([0.9, 0.8]);
    for action in 0..82 {
        let (value, state) = match action {
            64 => (-0.25, 1),
            65 => (0.0, 0),
            _ => (values[action as usize], states[action as usize]),
        };
        assert_eq!(mapped.value(action).to_bits(), value.to_bits());
        assert_eq!(mapped.state(action), state);
    }
}

#[test]
fn offboard_inactive_adapter_preserves_every_onboard_action() {
    let mut packet = Packet::distinct();
    let values = packet.values;
    let states = packet.states;
    let controls = PlayerControls::default();
    let mut mapped = controls.simulation_actions(&mut packet);
    for action in 0..82 {
        assert_eq!(
            mapped.value(action).to_bits(),
            values[action as usize].to_bits()
        );
        assert_eq!(mapped.state(action), states[action as usize]);
    }
}

#[test]
fn offboard_sample_reaches_derived_history_and_raw_tick_clears_override() {
    let mut packet = Packet::distinct();
    let mut controls = PlayerControls::default();
    controls.update(
        &mut SimulationActions {
            source: &mut packet,
            offboard_axes: Some([-0.25, 0.75]),
        },
        1.0 / 60.0,
        0.1,
        0,
    );
    assert_eq!(controls.controller.words()[7], (-0.25f32).to_bits());
    assert_eq!(controls.controller.words()[8], 0.75f32.to_bits());
    controls.offboard_axes = Some([-0.25, 0.75]);
    controls.update(&mut packet, 1.0 / 60.0, 0.1, 0);
    assert_eq!(controls.offboard_axes, None);
    assert_eq!(controls.controller.words()[0], (-0.25f32).to_bits());
    assert_eq!(controls.controller.words()[1], 0.75f32.to_bits());
    assert_eq!(controls.controller.words()[7], packet.values[64].to_bits());
    assert_eq!(controls.controller.words()[8], packet.values[65].to_bits());
}
