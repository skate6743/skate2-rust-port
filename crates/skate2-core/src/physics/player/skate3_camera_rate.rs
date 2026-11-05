//! Optional S3 camera time effects; physical integration keeps its fixed step.
use std::sync::Mutex;

use skate3_core::camera::SimulationRateRequest;

use crate::simulation_sim_rate::SimulationSimRate;

pub(super) fn submit(
    clock: &Mutex<SimulationSimRate>,
    requests: &[SimulationRateRequest],
    enabled: bool,
    tick: u64,
) -> Result<(), String> {
    for request in requests {
        if enabled {
            clock
                .lock()
                .map_err(|_| "native simulation clock lock")?
                .send(super::host_sim_rate_request(*request))
                .map_err(|error| error.to_string())?;
        }
        crate::diagnostics::test_log::event(
            if enabled {
                "skate3_sim_rate"
            } else {
                "skate3_camera_rate_suppressed"
            },
            format_args!(
                "tick={tick} rate={} source_ticks={} enabled={enabled} clock_owner=world",
                request.timestep, request.ticks,
            ),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate3_core::{
        camera::{SlowMotionController, SlowMotionSettings},
        point_graph::PointGraph,
    };

    fn clock() -> Mutex<SimulationSimRate> {
        let mut clock = SimulationSimRate::construct();
        clock.bind_game_info(crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field());
        Mutex::new(clock)
    }

    fn actual_camera_requests() -> Vec<SimulationRateRequest> {
        let data = super::super::stock_collections().expect("stock source collections");
        let values = data
            .words::<32>("slowmotion_controller", "default", "timescale")
            .unwrap()
            .map(f32::from_bits);
        let settings = SlowMotionSettings {
            timescale: PointGraph {
                x: std::array::from_fn(|i| values[i]),
                y: std::array::from_fn(|i| values[i + 16]),
            },
            fps_at_scale_one: data
                .float("slowmotion_controller", "default", "fps_at_scale_one")
                .unwrap(),
        };
        let (mut controller, begin) = SlowMotionController::begin(settings);
        let mut requests = vec![begin];
        for _ in 0..150 {
            requests.push(controller.update(1. / 60., 1.5, settings));
        }
        requests.push(SlowMotionController::end());
        assert!(
            requests.iter().any(|r| r.timestep > 1. / 40.),
            "actual camera curve never requested slow motion"
        );
        requests
    }

    #[test]
    fn actual_camera_curve_is_optional_and_does_not_cancel_native_script_rates() {
        let requests = actual_camera_requests();
        for enabled in [false, true] {
            let clock = clock();
            for (tick, request) in requests.iter().enumerate() {
                submit(&clock, std::slice::from_ref(request), enabled, tick as u64).unwrap();
                let mut clock = clock.lock().unwrap();
                clock.try_dispatch().unwrap();
                let expected = if enabled {
                    (1.0_f32 / request.timestep + 0.5) as i32
                } else {
                    60
                };
                assert_eq!(clock.timer_hz(), expected);
                clock.try_count_down().unwrap();
            }
        }
        let clock = clock();
        clock
            .lock()
            .unwrap()
            .send(crate::simulation_sim_rate::SetSimRate {
                sim_time: 1. / 30.,
                ticks_of_slow_motion: 10,
            })
            .unwrap();
        // Even the source camera's normal-rate End must not erase a native
        // story/script rate while the camera mod is disabled.
        submit(&clock, &requests, false, 200).unwrap();
        let mut clock = clock.lock().unwrap();
        clock.try_dispatch().unwrap();
        for _ in 0..10 {
            clock.try_count_down().unwrap();
            assert_eq!(clock.timer_hz(), 30);
        }
        clock.try_count_down().unwrap();
        assert_eq!(clock.timer_hz(), 60);
    }

    #[test]
    fn disabling_during_a_jump_expires_the_previous_short_request() {
        let requests = actual_camera_requests();
        let slow = *requests.iter().find(|r| r.timestep > 1. / 40.).unwrap();
        let clock = clock();
        submit(&clock, &[slow], true, 1).unwrap();
        {
            let mut clock = clock.lock().unwrap();
            clock.try_dispatch().unwrap();
            clock.try_count_down().unwrap();
            assert!(clock.timer_hz() < 40);
        }
        submit(&clock, &[slow, SlowMotionController::end()], false, 2).unwrap();
        let mut clock = clock.lock().unwrap();
        clock.try_dispatch().unwrap();
        clock.try_count_down().unwrap();
        assert_eq!(clock.timer_hz(), 60);
    }
}
