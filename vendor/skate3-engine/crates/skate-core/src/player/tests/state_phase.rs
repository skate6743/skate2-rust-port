use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    State,
    Controller,
    Mode(u32),
    BoardTouch,
    Forces,
    FixedStep,
}

#[derive(Default)]
struct Services(Vec<Event>);

impl StatePhaseServices for Services {
    fn update_current_state_vtable_8(&mut self) {
        self.0.push(Event::State);
    }

    fn update_controller_82d75f00(&mut self) {
        self.0.push(Event::Controller);
    }

    fn update_controller_mode_1_82d751d8(&mut self) {
        self.0.push(Event::Mode(1));
    }

    fn update_controller_mode_2_82d750f0(&mut self) {
        self.0.push(Event::Mode(2));
    }

    fn update_controller_mode_3_82d75bb8(&mut self) {
        self.0.push(Event::Mode(3));
    }

    fn update_controller_mode_4_82d75d58(&mut self) {
        self.0.push(Event::Mode(4));
    }

    fn touch_skateboard_vtable_116(&mut self) {
        self.0.push(Event::BoardTouch);
    }

    fn apply_skateboard_force_queue_82c03718(&mut self) {
        self.0.push(Event::Forces);
    }

    fn update_skateboard_fixed_step_cache_82db61f0(&mut self) {
        self.0.push(Event::FixedStep);
    }
}

#[test]
fn active_controller_mode_runs_between_state_and_board_work() {
    for mode in 1..=4 {
        let mut fields = StatePhaseFields {
            elapsed_1344: 1.0,
            timestep_2604: f32::from_bits(0x3C88_8889),
            controller_state_448: mode,
            controller_system_on_452: true,
        };
        let mut services = Services::default();

        run_state_phase(&mut fields, &mut services);

        assert_eq!(
            services.0,
            [
                Event::State,
                Event::Controller,
                Event::Mode(mode),
                Event::BoardTouch,
                Event::Forces,
                Event::FixedStep,
            ]
        );
        assert_eq!(
            fields.elapsed_1344.to_bits(),
            (1.0f32 + f32::from_bits(0x3C88_8889)).to_bits()
        );
    }
}

#[test]
fn inactive_or_unknown_controller_skips_only_controller_calls() {
    for (system_on, mode) in [(false, 2), (true, 0), (true, 5)] {
        let mut fields = StatePhaseFields {
            elapsed_1344: -0.0,
            timestep_2604: 0.0,
            controller_state_448: mode,
            controller_system_on_452: system_on,
        };
        let mut services = Services::default();

        run_state_phase(&mut fields, &mut services);

        let expected = if system_on {
            vec![
                Event::State,
                Event::Controller,
                Event::BoardTouch,
                Event::Forces,
                Event::FixedStep,
            ]
        } else {
            vec![
                Event::State,
                Event::BoardTouch,
                Event::Forces,
                Event::FixedStep,
            ]
        };
        assert_eq!(services.0, expected);
        assert_eq!(fields.elapsed_1344.to_bits(), 0);
    }
}
