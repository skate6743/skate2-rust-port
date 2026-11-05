//! SlowMotionController TU382BB1DD8/82BB1EE0/82BB20A8. The host owns
//! message delivery; this retains the curve, latching and timestep calculation.
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct SlowMotionSettings {
    pub timescale: PointGraph<16>,
    pub fps_at_scale_one: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct SimulationRateRequest {
    pub timestep: f32,
    pub ticks: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct SlowMotionController {
    elapsed: f32,
    air_duration: f32,
    time_before_prediction: f32,
    latched_prediction: bool,
}
impl SlowMotionController {
    pub fn begin(settings: SlowMotionSettings) -> (Self, SimulationRateRequest) {
        (
            Self {
                elapsed: 0.0,
                air_duration: 0.0,
                time_before_prediction: 0.0,
                latched_prediction: false,
            },
            settings.request(0.0),
        )
    }
    pub fn update(
        &mut self,
        dt: f32,
        air_duration: f32,
        settings: SlowMotionSettings,
    ) -> SimulationRateRequest {
        self.elapsed += dt;
        //82BB1FC8 uses ble (not GT), which also takes the unordered case.
        let progress = if !(air_duration > 0.0) {
            if !self.latched_prediction {
                self.time_before_prediction += dt;
            }
            f32::from_bits(0x3dcccccd)
        } else {
            if !self.latched_prediction {
                self.air_duration = air_duration;
                self.latched_prediction = true;
            }
            self.elapsed / (self.time_before_prediction + self.air_duration)
        };
        settings.request(progress)
    }
    pub fn end() -> SimulationRateRequest {
        SimulationRateRequest {
            timestep: f32::from_bits(0x3c888889),
            ticks: 0,
        }
    }
}
impl SlowMotionSettings {
    fn request(self, progress: f32) -> SimulationRateRequest {
        let curve = self.timescale.evaluate(progress);
        SimulationRateRequest {
            timestep: 1.0 / curve.mul_add(self.fps_at_scale_one - 60.0, 60.0),
            ticks: 1,
        }
    }
}
