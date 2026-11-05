//! Offline tick cadence. Original TU3 Simulation::OnRequestSetSimRate82857EC8
//! sets a wall-clock timer; it does not change the physical integration step.
use skate3_core::camera::SimulationRateRequest;
use std::time::Duration;

const NORMAL_STEP: f32 = f32::from_bits(0x3c888889);

#[derive(Clone, Copy, Debug)]
pub(super) struct SimulationClock {
    ticks_until_reset: u32,
    timer_period: Duration,
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self {
            ticks_until_reset: 0,
            timer_period: timer_period(60),
        }
    }
}

impl SimulationClock {
    pub fn apply(&mut self, request: SimulationRateRequest) -> Result<(), String> {
        //82857F44..7C rounds the reciprocal to an integer timer frequency.
        let frequency = (1.0_f32 / request.timestep + 0.5).trunc();
        if !frequency.is_finite() || frequency < 1.0 || frequency > i32::MAX as f32 {
            return Err(format!(
                "Invalid simulation-rate request: {}",
                request.timestep
            ));
        }
        let frequency = frequency as i32;
        if 10_000_000 / frequency == 0 {
            return Err("Simulation-rate request has a zero timer period".into());
        }
        self.timer_period = timer_period(frequency);
        self.ticks_until_reset = if request.timestep == NORMAL_STEP {
            0
        } else if (request.ticks as i32) > 0 {
            request.ticks.wrapping_add(1)
        } else {
            //TU3 hardcodes180 at82857F34; S2 used a debug setting here.
            180
        };
        Ok(())
    }

    pub fn finish_tick(&mut self) {
        //SimUpdate2 8285A694..A6C8 runs after the camera publication. Pending
        //camera messages are dispatched at the start of the following tick.
        self.ticks_until_reset = self.ticks_until_reset.wrapping_sub(1);
        if self.ticks_until_reset == 0 {
            self.timer_period = timer_period(60);
        }
    }

    pub fn period(&self) -> Duration {
        self.timer_period
    }
}

fn timer_period(frequency: i32) -> Duration {
    //Original timer setter82966910..2C: integer division in100ns units.
    Duration::from_nanos((10_000_000 / frequency) as u64 * 100)
}

#[cfg(test)]
#[path = "tests/clock.rs"]
mod tests;
