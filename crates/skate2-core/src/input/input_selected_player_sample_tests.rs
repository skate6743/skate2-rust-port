use super::*;
use crate::{
    input_cvar_runtime::InputCvarReferences,
    input_map::SelectedInputMap,
    input_pad::device::tests::{Platform, Remap},
    input_pad::device_manager::{InputDeviceManager, selected_runtime::SelectedDeviceInputEvents},
    input_pad_cache::InputPadCache,
    reckoning_numeric::HostReckoningNumeric,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
};
use std::sync::Arc;
type Runtime = SelectedDeviceInputRuntime<u32, HostSkeletonCollisionNumeric, Remap>;

fn sampled_runtime() -> Runtime {
    sampled_runtime_with(None)
}
fn sampled_runtime_with(controller_zero: Option<crate::input_pad::XboxPadSample>) -> Runtime {
    let mut devices = InputDeviceManager::new();
    devices.initialize().unwrap();
    let mut platform = Platform::new();
    platform.scripted_controller_zero = controller_zero;
    for device in devices.devices_mut().unwrap() {
        device
            .reset_actuators(
                &mut platform,
                &InputCvarReferences::new_static_initialized(),
                &mut HostReckoningNumeric,
            )
            .unwrap();
    }
    devices.set_external_remap(Some(&42));
    let map =
        Arc::new(SelectedInputMap::parse(&crate::input_map::tests::config("Button0")).unwrap());
    let mut runtime = Runtime::from_owners(
        devices,
        InputPadCache::new(),
        InputPadCache::new(),
        map,
        HostSkeletonCollisionNumeric,
        Remap,
    );
    runtime.poll_hardware(&mut platform).unwrap();
    runtime
}
struct Pause {
    answer: Result<bool, &'static str>,
    calls: usize,
}
impl PlayerInputPause for Pause {
    type Error = &'static str;
    fn try_is_simulation_active(&mut self) -> Result<bool, Self::Error> {
        self.calls += 1;
        self.answer
    }
}
struct Proxy {
    answer: Result<u32, &'static str>,
    calls: usize,
}
impl PlayerInputProxyFrame for Proxy {
    type Error = &'static str;
    fn try_next_generation_frame(&mut self) -> Result<u32, Self::Error> {
        self.calls += 1;
        self.answer
    }
}

#[test]
fn focused_sample_uses_actual_raw_owner_and_loader_without_pause_proxy_reads() {
    let runtime = sampled_runtime();
    let mut pause = Pause {
        answer: Err("unavailable pause"),
        calls: 0,
    };
    let mut proxy = Proxy {
        answer: Err("unavailable proxy"),
        calls: 0,
    };
    let sample = runtime
        .try_player_sample(
            &PlayerLoaderSlot::new(7),
            0,
            &PhysicalGameInfo::construct_network_field(),
            true,
            None,
            &mut pause,
            &mut proxy,
        )
        .unwrap();
    let request = sample.frame_request(0x28, true);
    assert_eq!(request.receiver_id, 17);
    assert_eq!(request.update_mask, 0x28);
    assert!(request.add_to_crc);
    assert!(request.input.allow_sensor); // Actual constructor network field is false.
    assert!(request.input.has_focus);
    assert_eq!(request.input.sample.count, 24);
    assert_eq!(
        request.input.sample.values.map(f32::to_bits),
        runtime
            .raw_current_cache_line(0)
            .unwrap()
            .values
            .map(f32::to_bits)
    );
    assert_eq!((pause.calls, proxy.calls), (0, 0));
}

#[test]
fn unfocused_payload_keeps_count_clears_all_words_and_samples_proxy_lazily_unsigned() {
    let runtime = sampled_runtime();
    for (active, frame, focused, proxy_calls) in [
        (false, u32::MAX, true, 0),
        (true, 0, true, 1),
        (true, 1, true, 1),
        (true, 2, false, 1),
        (true, u32::MAX, false, 1),
    ] {
        let mut pause = Pause {
            answer: Ok(active),
            calls: 0,
        };
        let mut proxy = Proxy {
            answer: Ok(frame),
            calls: 0,
        };
        let sample = runtime
            .try_player_sample(
                &PlayerLoaderSlot::new(u32::MAX),
                0,
                &PhysicalGameInfo::construct_network_field(),
                false,
                None,
                &mut pause,
                &mut proxy,
            )
            .unwrap();
        let request = sample.frame_request(1, false);
        assert_eq!(request.receiver_id, 9);
        assert_eq!(request.input.sample.count, 24);
        assert_eq!(request.input.sample.values.map(f32::to_bits), [0; 24]);
        assert_eq!(request.input.has_focus, focused);
        assert_eq!((pause.calls, proxy.calls), (1, proxy_calls));
    }
}

#[test]
fn unavailable_pause_or_proxy_is_a_lazy_error_and_device_failure_precedes_both() {
    let runtime = sampled_runtime();
    let loader = PlayerLoaderSlot::new(3);
    let info = PhysicalGameInfo::construct_network_field();
    let mut pause = Pause {
        answer: Err("pause"),
        calls: 0,
    };
    let mut proxy = Proxy {
        answer: Err("proxy"),
        calls: 0,
    };
    assert!(matches!(
        runtime.try_player_sample(&loader, 4, &info, false, None, &mut pause, &mut proxy),
        Err(SelectedPlayerSampleError::Device(
            InputPadCacheError::DeviceIndex(4)
        ))
    ));
    assert_eq!((pause.calls, proxy.calls), (0, 0));
    assert!(matches!(
        runtime.try_player_sample(&loader, 0, &info, false, None, &mut pause, &mut proxy),
        Err(SelectedPlayerSampleError::Pause("pause"))
    ));
    assert_eq!((pause.calls, proxy.calls), (1, 0));
    pause.answer = Ok(true);
    assert!(matches!(
        runtime.try_player_sample(&loader, 0, &info, false, None, &mut pause, &mut proxy),
        Err(SelectedPlayerSampleError::Proxy("proxy"))
    ));
    assert_eq!((pause.calls, proxy.calls), (2, 1));
}

#[test]
fn unfocused_sample_reads_retained_pause_owner_and_its_live_top_state() {
    let runtime = sampled_runtime();
    let mut pause = crate::selected_pause_manager::SelectedPauseManager::construct();
    let shared = Arc::clone(&pause);
    let mut proxy = Proxy {
        answer: Ok(2),
        calls: 0,
    };
    let loader = PlayerLoaderSlot::new(0);
    let info = PhysicalGameInfo::construct_network_field();
    let sample = runtime
        .try_player_sample(&loader, 0, &info, false, None, &mut pause, &mut proxy)
        .unwrap();
    assert!(!sample.input.has_focus);
    assert_eq!(sample.input.sample.values.map(f32::to_bits), [0; 24]);
    assert_eq!(proxy.calls, 1);
    shared.try_push(1).unwrap();
    let sample = runtime
        .try_player_sample(&loader, 0, &info, false, None, &mut pause, &mut proxy)
        .unwrap();
    assert!(sample.input.has_focus);
    assert_eq!(sample.input.sample.values.map(f32::to_bits), [0; 24]);
    assert_eq!(proxy.calls, 1);
}

fn frames(rows: [[f32; 4]; 3]) -> [SkeletonRootVector; 3] {
    rows.map(SkeletonRootVector::known)
}
const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];
/// `GP_LStickX` and `GP_LStickY` (LStickR - LStickL, LStickU - LStickD) after the
/// off-board stick of a line whose four stick buttons are `buttons`.
fn off_board_stick(frames: &OffBoardStickFrames, buttons: [f32; 4]) -> (f32, f32) {
    let mut values = [0.0; 24];
    values[16..20].copy_from_slice(&buttons);
    frames
        .try_apply(&mut values, &mut HostReckoningNumeric)
        .unwrap();
    (values[16] - values[17], values[18] - values[19])
}

#[test]
fn off_board_the_stick_is_where_the_camera_sees_it_in_the_skaters_frame() {
    let behind = OffBoardStickFrames {
        camera: frames(IDENTITY),
        skater: frames(IDENTITY),
    };
    // The camera's first row is the screen's left: a push right is the skater's -X.
    assert_eq!(off_board_stick(&behind, [1.0, 0.0, 0.0, 0.0]), (-1.0, 0.0));
    assert_eq!(off_board_stick(&behind, [0.0, 0.0, 0.0, 1.0]), (0.0, -1.0));
    assert_eq!(off_board_stick(&behind, [0.0; 4]), (0.0, 0.0));
    // Facing the camera the same push is +X, a pull toward it is forward, and a
    // half push becomes a full one.
    let facing = OffBoardStickFrames {
        camera: frames(IDENTITY),
        skater: frames([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
        ]),
    };
    assert_eq!(off_board_stick(&facing, [0.5, 0.0, 0.0, 0.0]), (1.0, 0.0));
    assert_eq!(off_board_stick(&facing, [0.0, 0.0, 0.0, 1.0]), (0.0, 1.0));
    // A camera looking down still pushes level: its view loses its height.
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let looking_down = OffBoardStickFrames {
        camera: frames([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, s, s, 0.0],
            [0.0, -s, s, 0.0],
        ]),
        skater: frames(IDENTITY),
    };
    let (x, y) = off_board_stick(&looking_down, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(x, 0.0);
    assert!((y - 1.0).abs() < 1e-6);
}

#[test]
fn an_off_board_line_turns_before_the_focus_test() {
    let runtime = sampled_runtime_with(Some(crate::input_pad::XboxPadSample {
        buttons: 0,
        left_trigger: 0,
        right_trigger: 0,
        left_x: i16::MAX,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    }));
    let raw = runtime.raw_current_cache_line(0).unwrap().values;
    assert!(raw[16] > 0.0 && raw[17] == 0.0);
    let behind = OffBoardStickFrames {
        camera: frames(IDENTITY),
        skater: frames(IDENTITY),
    };
    let mut turned = raw;
    behind
        .try_apply(&mut turned, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(turned[16], 0.0);
    assert!((turned[17] - 1.0).abs() < 1e-6);
    let info = PhysicalGameInfo::construct_network_field();
    let mut pause = Pause {
        answer: Ok(false),
        calls: 0,
    };
    let mut proxy = Proxy {
        answer: Err("unavailable proxy"),
        calls: 0,
    };
    let loader = PlayerLoaderSlot::new(0);
    let sample = runtime
        .try_player_sample(&loader, 0, &info, true, Some(&behind), &mut pause, &mut proxy)
        .unwrap();
    assert_eq!(
        sample.input.sample.values.map(f32::to_bits),
        turned.map(f32::to_bits)
    );
    // Unfocused, the turned line is cleared like any other.
    let sample = runtime
        .try_player_sample(&loader, 0, &info, false, Some(&behind), &mut pause, &mut proxy)
        .unwrap();
    assert_eq!(sample.input.sample.values.map(f32::to_bits), [0; 24]);
}
