//! TU3 normal-camera positioner 82E01948 and its complete collision/geometry
//! callees. The caller supplies recovered state/configuration and the native
//! six-fat-line backend; this module does not choose camera settings.

use super::vector_tracker::{length, refined_reciprocal};
use super::{ScalarTracker, ScalarTrackerParameters};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FatLine {
    pub start: [f32; 4],
    pub end: [f32; 4],
    pub radius: f32,
}

/// Native 48-byte result: vectors0/16, fraction32, hit byte36, surface word40.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FatLineResult {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub fraction: f32,
    pub hit: u8,
    pub surface: u32,
}

/// Camera server slots+8/+12, implemented natively by82DF31C0/82DF3290.
/// Results preserve segment order; native submission always contains six lines.
pub trait PositionerCollisionProvider {
    fn submit(&mut self, lines: [FatLine; 6], context: u32);
    fn results(&mut self) -> [FatLineResult; 6];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionerConfig {
    pub anchor: [f32; 4],
    pub offset: [f32; 4],
    pub reference_heading: f32,
    pub reference_elevation: f32,
    pub heading: f32,
    pub elevation: f32,
    pub distance: f32,
    pub minimum_distance: f32,
    pub collision_enabled: u8,
    pub bypass_distance_tracker: u8,
    pub clamp_height: u8,
    pub floor_height: f32,
    pub speed_clamp: f32,
    pub acceleration_clamp: f32,
    pub smoothing: f32,
    pub query_starts: [[f32; 4]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Positioner {
    pub distance_tracker: ScalarTracker,
    pub tracker_parameters: ScalarTrackerParameters,
    pub offset: [f32; 4],
    pub previous_valid_position: [f32; 4],
    pub reference_position: [f32; 4],
    pub collision_clear_time: f32,
    pub config: PositionerConfig,
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub distance: f32,
    pub heading: f32,
    pub elevation: f32,
    pub radius: f32,
    pub available_distance: f32,
    pub flags: u8,
}

impl Positioner {
    /// Complete82E01948. Negative/zero dt follows native tracker/velocity gates.
    pub fn update(
        &mut self,
        dt: f32,
        context: u32,
        config: PositionerConfig,
        collision: &mut impl PositionerCollisionProvider,
    ) -> u32 {
        self.config = config;
        self.offset = config.offset;
        if self.collision_clear_time < 5.0 {
            let scale = self.collision_clear_time * f32::from_bits(0x3e4ccccd);
            self.offset = self.offset.map(|v| v * scale);
        }
        if config.bypass_distance_tracker != 0 {
            self.distance_tracker.target = config.distance;
            self.distance_tracker.position = config.distance;
        } else {
            let acceleration = if self.distance_tracker.position < f32::from_bits(0x3f866666) {
                100.0
            } else {
                config.acceleration_clamp
            };
            self.tracker_parameters.acceleration_clamp_min = acceleration;
            self.tracker_parameters.acceleration_clamp_max = acceleration;
            self.tracker_parameters.speed_clamp = config.speed_clamp;
            self.tracker_parameters.smoothing_min = config.smoothing;
            self.tracker_parameters.smoothing_max = config.smoothing;
            self.distance_tracker
                .update(dt, config.distance, self.tracker_parameters);
        }
        self.radius = f32::from_bits(0x3e051eb8);
        let previous = self.position;
        let result = self.resolve(context, collision);
        if dt > 0.0 {
            let inverse = refined_reciprocal(dt);
            self.velocity = core::array::from_fn(|i| inverse * (self.position[i] - previous[i]));
        }
        if self.flags & 0xc0 != 0 {
            self.collision_clear_time = 0.0;
        } else {
            self.collision_clear_time += dt;
        }
        result
    }

    /// Complete82E01BA0. Query status 3 preserves previous-valid state; the
    /// current complete82E01EA8 body produces only statuses0,1,2.
    pub fn resolve(
        &mut self,
        context: u32,
        collision: &mut impl PositionerCollisionProvider,
    ) -> u32 {
        self.available_distance = self.config.distance;
        self.flags = (self.flags & 0xc1) | 0x0c;
        self.heading = self.config.heading;
        self.elevation = self.config.elevation;
        self.distance = self.distance_tracker.position;
        self.position = self.position_at(self.elevation, self.heading, self.distance);
        let result = self.resolve_collision(context, collision);
        if result == 1 || result == 2 {
            let distance = length(core::array::from_fn(|i| {
                self.position[i] - self.config.anchor[i]
            }));
            self.distance_tracker.target = distance;
            self.distance_tracker.position = distance;
            self.distance_tracker.velocity = 0.0;
            self.distance_tracker.acceleration = 0.0;
            self.distance = distance;
        }
        if result == 3 {
            self.position = self.previous_valid_position;
            self.flags &= !8;
        } else {
            self.previous_valid_position = self.position;
            self.flags = (self.flags & !0x20) | if result == 2 { 0x20 } else { 0 };
        }
        self.flags = (self.flags & 0x3f) | if result != 0 { 0x80 } else { 0 };
        result
    }

    /// Complete82E01D30. Submit happens even when collision_enabled is zero.
    pub fn submit_collision(
        &mut self,
        context: u32,
        collision: &mut impl PositionerCollisionProvider,
    ) {
        self.reference_position = self.position_at(
            self.config.reference_elevation,
            self.config.reference_heading,
            self.config.distance,
        );
        let lines = core::array::from_fn(|i| FatLine {
            start: self.config.query_starts[i % 3],
            end: if i < 3 {
                self.position
            } else {
                self.reference_position
            },
            radius: self.radius,
        });
        collision.submit(lines, context);
    }

    /// Complete82E01EA8, preserving first-three/last-three result roles and order.
    pub fn resolve_collision(
        &mut self,
        context: u32,
        collision: &mut impl PositionerCollisionProvider,
    ) -> u32 {
        self.submit_collision(context, collision);
        let results = collision.results();
        if self.config.collision_enabled != 0 {
            self.flags |= 2;
            self.available_distance = 0.0;
            for result in &results[3..6] {
                if result.hit != 0 {
                    let point = project_hit(
                        self.reference_position,
                        self.config.query_starts[0],
                        *result,
                    );
                    let distance =
                        length(core::array::from_fn(|i| point[i] - self.config.anchor[i]));
                    if !(self.available_distance - distance >= 0.0) {
                        self.available_distance = distance;
                    }
                } else {
                    self.available_distance = self.config.distance;
                    self.flags &= !2;
                }
            }
            if results[..3].iter().all(|r| r.hit != 0) {
                self.position = project_hit(self.position, self.config.query_starts[0], results[0]);
                let distance = length(core::array::from_fn(|i| {
                    self.position[i] - self.config.query_starts[0][i]
                }));
                return if distance < self.radius + self.config.minimum_distance {
                    2
                } else {
                    1
                };
            }
        }
        0
    }

    fn position_at(&self, elevation: f32, heading: f32, distance: f32) -> [f32; 4] {
        position_from_angles(
            self.config.anchor,
            self.offset,
            elevation,
            heading,
            distance,
            self.config.clamp_height,
            self.radius + self.config.floor_height,
        )
    }
}

/// Complete82E020F0. Invalid hit positions or a zero hit byte preserve the end.
pub fn project_hit(end: [f32; 4], first_query_start: [f32; 4], result: FatLineResult) -> [f32; 4] {
    if result.position[..3].iter().all(|v| !v.is_nan()) && result.hit != 0 {
        let lower = if -result.fraction >= 0.0 {
            0.0
        } else {
            result.fraction
        };
        let fraction = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
        core::array::from_fn(|i| {
            (end[i] - first_query_start[i]).mul_add(fraction, first_query_start[i])
        })
    } else {
        end
    }
}

/// Complete82E02190, including native four-lane basis construction and ordering.
pub fn position_from_angles(
    anchor: [f32; 4],
    offset: [f32; 4],
    elevation: f32,
    heading: f32,
    distance: f32,
    clamp_height: u8,
    minimum_height: f32,
) -> [f32; 4] {
    let (s_e, c_e) = crate::trigonometry::sin_cos(-elevation);
    let (s_h, c_h) = crate::trigonometry::sin_cos(-heading);
    // 822FB890 permutation and vrlimi lane insertion, in guest memory order.
    let first = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, c_e, s_e, 0.0],
        [0.0, -s_e, c_e, 0.0],
    ];
    let second = [
        [c_h, 0.0, -s_h, c_h],
        [0.0, 1.0, 0.0, 0.0],
        [s_h, 0.0, c_h, s_h],
    ];
    let product: [[f32; 4]; 3] = core::array::from_fn(|column| {
        core::array::from_fn(|i| {
            let value = second[0][i] * first[column][0];
            let value = second[1][i].mul_add(first[column][1], value);
            second[2][i].mul_add(first[column][2], value)
        })
    });
    let mut point = core::array::from_fn(|i| {
        let value = product[0][i] * 0.0;
        let value = product[1][i].mul_add(0.0, value);
        let value = product[2][i].mul_add(distance, value);
        (value + offset[i]) + anchor[i]
    });
    if clamp_height != 0 && minimum_height > point[1] {
        point[1] = minimum_height;
    }
    point
}
