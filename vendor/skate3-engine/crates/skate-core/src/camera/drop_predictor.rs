//! Native terrain drop prediction82DF84F8/89B0/8BC0/8740.
//! Retains one update of query history: a new batch is submitted only after the
//! previous batch has been consumed. Host execution may be synchronous.
use super::vector_tracker::length;
use super::{FatLine, FatLineResult};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropSettings {
    /// camera_droppredictor/default fields bound8252EF84..8252EFA4.
    pub minimum_test_distance: f32,
    pub total_test_time: f32,
    pub maximum_test_distance: f32,
    pub maximum_drop_distance: f32,
}

pub trait DropCollisionProvider {
    fn query(&mut self, lines: [FatLine; 10], context: u32) -> Result<[FatLineResult; 10], String>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Probe {
    start: [f32; 4],
    end: [f32; 4],
    normal: [f32; 4],
    depth: f32,
    valid: bool,
    hit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropPredictor {
    pub horizontal_velocity: [f32; 4],
    pub steepest_normal: [f32; 4],
    pub has_normal: bool,
    pub elevation: f32,
    pub distance_offset: f32,
    pub target_elevation: f32,
    spacing: f32,
    selected: usize,
    probes: [Probe; 5],
    pending: Option<([FatLine; 10], [FatLineResult; 10])>,
}

impl DropPredictor {
    pub fn new() -> Self {
        Self {
            horizontal_velocity: [0.0; 4],
            steepest_normal: [0.0; 4],
            has_normal: false,
            elevation: 0.0,
            distance_offset: 0.0,
            target_elevation: 0.0,
            spacing: 0.0,
            selected: 0,
            probes: [Probe {
                start: [0.0; 4],
                end: [0.0; 4],
                normal: [0.0; 4],
                depth: 0.0,
                valid: false,
                hit: false,
            }; 5],
            pending: None,
        }
    }

    /// Reset82DF8440 preserves the five probe records and cancels their pending
    /// query. The following disabled/reset update clears probe valid/depth.
    pub fn reset(&mut self) {
        self.horizontal_velocity = [0.0; 4];
        self.steepest_normal = [0.0; 4];
        self.has_normal = false;
        self.elevation = 0.0;
        self.distance_offset = 0.0;
        self.target_elevation = 0.0;
        self.spacing = 0.0;
        self.selected = 0;
        self.pending = None;
    }

    /// Complete82E00B60 velocity selection, consuming the previous rig velocity.
    pub fn prediction_velocity(
        reset: bool,
        transform: [[f32; 4]; 4],
        rig_velocity: [f32; 4],
        settings: DropSettings,
    ) -> [f32; 4] {
        if reset || settings.minimum_test_distance > length(rig_velocity) {
            transform[2].map(|v| v * settings.minimum_test_distance)
        } else {
            rig_velocity
        }
    }

    pub fn update(
        &mut self,
        position: [f32; 4],
        velocity: [f32; 4],
        enabled: bool,
        offboard: bool,
        context: u32,
        settings: DropSettings,
        query: &mut impl DropCollisionProvider,
    ) -> Result<(), String> {
        self.horizontal_velocity = [velocity[0], 0.0, velocity[2], 0.0];
        if enabled {
            let span = length(self.horizontal_velocity) * settings.total_test_time;
            let span = if -span >= 0.0 { 0.0 } else { span };
            let span = if settings.maximum_test_distance - span >= 0.0 {
                span
            } else {
                settings.maximum_test_distance
            };
            self.spacing = span / 5.0;
            if let Some((lines, results)) = self.pending.take() {
                self.consume(lines, results, settings.maximum_drop_distance);
            }
            let lines = self.lines(position, settings.maximum_drop_distance);
            let results = query.query(lines, context)?;
            self.pending = Some((lines, results));
            let mut greatest_depth = 0.0;
            self.selected = 0;
            for (index, probe) in self.probes.iter().enumerate() {
                if !probe.valid {
                    break;
                }
                if probe.depth > greatest_depth {
                    self.selected = index;
                    greatest_depth = probe.depth;
                }
            }
        } else {
            self.selected = 0;
            for probe in &mut self.probes {
                probe.valid = false;
                probe.depth = 0.0;
            }
        }
        self.update_elevation(enabled, offboard);
        Ok(())
    }

    /// Native getter82DF8980.
    pub fn drop_depth(&self) -> f32 {
        let probe = self.probes[self.selected];
        if probe.valid { probe.depth } else { 0.0 }
    }

    fn lines(&self, position: [f32; 4], depth: f32) -> [FatLine; 10] {
        let direction = super::orientation_math::normalize(self.horizontal_velocity);
        core::array::from_fn(|index| {
            let point = core::array::from_fn(|i| {
                (direction[i] * self.spacing).mul_add((index / 2 + 1) as f32, position[i])
            });
            if index % 2 == 0 {
                FatLine {
                    start: position,
                    end: point,
                    radius: f32::from_bits(0x3c23d70a),
                }
            } else {
                FatLine {
                    start: point,
                    end: core::array::from_fn(|i| point[i] - [0.0, 1.0, 0.0, 0.0][i] * depth),
                    radius: f32::from_bits(0x3dcccccd),
                }
            }
        })
    }

    fn consume(&mut self, lines: [FatLine; 10], results: [FatLineResult; 10], maximum_depth: f32) {
        self.has_normal = false;
        for index in 0..5 {
            if results[2 * index].hit != 0 {
                for probe in &mut self.probes[index..] {
                    probe.valid = false;
                    probe.depth = 0.0;
                }
                break;
            }
            let line = lines[2 * index + 1];
            let result = results[2 * index + 1];
            let probe = &mut self.probes[index];
            *probe = Probe {
                start: line.start,
                end: line.end,
                normal: [0.0; 4],
                depth: maximum_depth,
                valid: true,
                hit: result.hit != 0,
            };
            if probe.hit {
                probe.depth = length(core::array::from_fn(|i| result.position[i] - line.start[i]));
                probe.end = result.position;
                probe.normal = result.normal;
                if !self.has_normal || self.steepest_normal[1].abs() > result.normal[1].abs() {
                    self.steepest_normal = result.normal;
                    self.has_normal = true;
                }
            }
        }
    }

    fn update_elevation(&mut self, enabled: bool, offboard: bool) {
        let probe = self.probes[self.selected];
        let mut angle = 0.0;
        if probe.valid {
            let distance = (self.selected + 1) as f32 * self.spacing;
            // Native angle wrapper applies one reciprocal refinement here; the
            // called82473B98 polynomial has its own independent reductions.
            let initial = crate::physics::native_arithmetic::reciprocal_estimate(distance);
            let inverse = initial.mul_add((-initial).mul_add(distance, 1.0), initial);
            angle = crate::input::angle::atan(probe.depth.mul_add(inverse, 0.0));
            let sign = probe.depth.to_bits() & 0x80000000;
            if 0.0 > distance {
                angle = f32::from_bits(0x40490fdb | sign) + angle;
            }
            if distance == 0.0 {
                angle = f32::from_bits(0x3fc90fdb | sign);
            }
        }
        let half_pi = f32::from_bits(0x3fc90fdb);
        if angle - half_pi >= 0.0 {
            angle = half_pi;
        }
        let limit = (if offboard { 40.0 } else { 70.0 }) * f32::from_bits(0x3c8efa35);
        if angle - limit >= 0.0 {
            angle = limit;
        }
        self.target_elevation = if enabled && angle >= f32::from_bits(0x3e860a92) {
            angle
        } else {
            0.0
        };
        let delta = (self.target_elevation - self.elevation).abs();
        let degrees = delta * f32::from_bits(0x42652ee1);
        let step = (degrees - 7.0).mul_add(f32::from_bits(0x3d3a2e8c), f32::from_bits(0x3dcccccd));
        let step = if f32::from_bits(0x3dcccccd) - step >= 0.0 {
            f32::from_bits(0x3dcccccd)
        } else {
            step
        };
        let step = if f32::from_bits(0x3fcccccd) - step >= 0.0 {
            step
        } else {
            f32::from_bits(0x3fcccccd)
        };
        if degrees > step {
            self.elevation = if self.elevation >= self.target_elevation {
                -step.mul_add(f32::from_bits(0x3c8efa35), -self.elevation)
            } else {
                step.mul_add(f32::from_bits(0x3c8efa35), self.elevation)
            };
        } else {
            self.elevation = self.target_elevation;
        }
    }
}
