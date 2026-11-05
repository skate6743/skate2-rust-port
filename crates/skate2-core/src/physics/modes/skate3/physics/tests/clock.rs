use super::*;
use skate3_core::{
    camera::{SlowMotionController, SlowMotionSettings},
    point_graph::PointGraph,
};

#[test]
fn camera_rate_requests_expire_and_preserve_message_order() {
    let mut clock = SimulationClock::default();
    let normal = clock.period();
    let slow = SimulationRateRequest {
        timestep: 1.0 / 35.4,
        ticks: 1,
    };
    clock.apply(slow).unwrap();
    clock.finish_tick();
    assert_eq!(clock.period(), Duration::from_nanos(28_571_400));
    clock.finish_tick();
    assert_eq!(clock.period(), normal);

    //An End followed by a new Begin must retain the new request.
    clock.apply(SlowMotionController::end()).unwrap();
    clock.apply(slow).unwrap();
    clock.finish_tick();
    assert_ne!(clock.period(), normal);
    clock.apply(SlowMotionController::end()).unwrap();
    clock.finish_tick();
    assert_eq!(clock.period(), normal);

    clock
        .apply(SimulationRateRequest { ticks: 0, ..slow })
        .unwrap();
    for _ in 0..179 {
        clock.finish_tick();
    }
    assert_ne!(clock.period(), normal);
    clock.finish_tick();
    assert_eq!(clock.period(), normal);
}

#[test]
fn camera_slow_motion_waits_for_a_positive_air_prediction() {
    let settings = SlowMotionSettings {
        timescale: PointGraph {
            x: std::array::from_fn(|i| i as f32 / 15.0),
            y: std::array::from_fn(|i| i as f32 / 15.0),
        },
        fps_at_scale_one: 35.0,
    };
    let (mut controller, _) = SlowMotionController::begin(settings);
    let waiting = controller.update(0.1, f32::NAN, settings);
    assert!((waiting.timestep - 1.0 / 57.5).abs() < 1e-6);
    let predicted = controller.update(0.1, 1.0, settings);
    let expected = 1.0 / (60.0 - 25.0 * (0.2 / 1.1));
    assert!((predicted.timestep - expected).abs() < 1e-6);
}

#[cfg(feature = "skate3-runtime")]
#[test]
fn host_timer_matches_skate3_rate_and_countdown_at_every_tick() {
    let mut source = SimulationClock::default();
    let mut host = crate::simulation_sim_rate::SimulationSimRate::construct();
    host.bind_game_info(crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field());
    // Explicit short lifetimes, Begin/End ordering, the source180-tick fallback,
    // and signed/nonpositive native tick words all retain their source cadence.
    for request in [
        SimulationRateRequest { timestep: 1. / 35.4, ticks: 1 },
        SimulationRateRequest { timestep: NORMAL_STEP, ticks: 0 },
        SimulationRateRequest { timestep: 1. / 30., ticks: 0 },
        SimulationRateRequest { timestep: 1. / 45., ticks: u32::MAX },
        SimulationRateRequest { timestep: NORMAL_STEP, ticks: 50 },
    ] {
        source.apply(request).unwrap();
        host.try_on_request_set_sim_rate(super::super::player::host_sim_rate_request(request)).unwrap();
        for _ in 0..185 {
            source.finish_tick();
            host.try_count_down().unwrap();
            let period = Duration::from_nanos((10_000_000 / host.timer_hz()) as u64 * 100);
            assert_eq!(period, source.period());
        }
    }
}
