//! Candidate construction82D6BF90 and query submission82D6CA58.
//! PredictionResults are produced by the shared82E099A0 collision query.
use super::air_launch::{Launch, V};
use crate::air::trajectory::{QueryRequest, Trajectory};
#[derive(Clone, Debug)]
pub struct Candidate {
    pub trajectory: Trajectory,
    pub skipped_frames: u32,
}
pub struct Settings {
    pub height: f32,
    pub sphere_radius: f32,
    pub start_index: u32,
}
pub struct Prepared {
    pub candidates: Vec<Candidate>,
    pub offset: V,
    pub original: Trajectory,
    pub launch_up: V,
}
pub fn prepare(launch: &Launch, gravity: V, s: &Settings) -> Prepared {
    let original = Trajectory {
        position: launch.position,
        velocity: launch.velocity,
        acceleration: gravity,
        duration: 2.,
    };
    let mut offset = [0., -(s.height - s.sphere_radius), 0., 0.];
    let mut correction = [0., 0.1, 0., 0.];
    if launch.has_target {
        correction = add(correction, sub(launch.target, add(launch.position, offset)));
    }
    offset = add(offset, correction);
    let base = sub(launch.velocity, correction);
    let base_speed = length(base);
    let count = launch.primary_count + launch.secondary_count;
    let mut velocities = vec![base];
    if count > 1 {
        let speed = length(launch.velocity);
        let (right, axis) = if speed < 0.001 {
            (scale(launch.forward, -1.), [0., 1., 0., 0.])
        } else {
            let axis = scale(launch.velocity, 1. / speed);
            let mut right = cross([0., 1., 0., 0.], axis);
            if length(right) < 0.1 {
                right = cross(axis, launch.forward);
            }
            (scale(right, 1. / length(right)), axis)
        };
        let vertical = cross(axis, right);
        let angles = launch.angles.map(|v| {
            let (s, c) = crate::trigonometry::sin_cos(v);
            s / c
        });
        let mut minimum_horizontal = horizontal_length(launch.velocity);
        for index in 1..launch.primary_count {
            let a = (1. - 2. * (index as f32 / (launch.primary_count - 1) as f32))
                * f32::from_bits(0x40490fdb);
            let (sin, cos) = crate::trigonometry::sin_cos(a);
            let v = madd(
                vertical,
                cos * if cos > 0. { angles[2] } else { angles[1] },
                madd(right, sin * angles[0], base),
            );
            let v = scale(v, base_speed / length(v));
            minimum_horizontal = minimum_horizontal.min(horizontal_length(v));
            velocities.push(v);
        }
        if launch.secondary_count != 0 {
            let n = horizontal_length(launch.secondary_velocity);
            let direction = if n > 0.01 {
                scale(
                    [
                        launch.secondary_velocity[0],
                        0.,
                        launch.secondary_velocity[2],
                        launch.secondary_velocity[3],
                    ],
                    1. / n,
                )
            } else {
                unit(launch.forward)
            };
            let step = minimum_horizontal.max(1.).min(4.) / (launch.secondary_count + 1) as f32;
            let mut speed = 0.;
            for _ in 0..launch.secondary_count {
                speed += step;
                velocities.push(madd(
                    direction,
                    speed,
                    [0., sqrt(f32::from_bits(0x41a95811)), 0., 0.],
                ));
            }
        }
    }
    let time = s.start_index as f32 * f32::from_bits(0x3c888889);
    let position = add(launch.position, offset);
    let candidates = velocities
        .into_iter()
        .take(count as usize)
        .map(|velocity| {
            let trajectory = Trajectory {
                position,
                velocity,
                acceleration: gravity,
                duration: 2.,
            };
            Candidate {
                trajectory: Trajectory {
                    position: trajectory.position_at(time),
                    velocity: trajectory.velocity_at(time),
                    ..trajectory
                },
                skipped_frames: s.start_index,
            }
        })
        .collect();
    Prepared {
        candidates,
        offset,
        original,
        launch_up: launch.up,
    }
}
impl Candidate {
    pub fn request(&self, s: &Settings) -> QueryRequest {
        QueryRequest {
            trajectory: self.trajectory,
            radius: s.sphere_radius,
            start_error: 0.25,
            end_error: f32::from_bits(0x3f99999a),
        }
    }
}
fn add(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(v: V, s: f32) -> V {
    v.map(|x| x * s)
}
fn madd(v: V, s: f32, b: V) -> V {
    std::array::from_fn(|i| v[i].mul_add(s, b[i]))
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
fn dot(a: V, b: V) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
fn sqrt(q: f32) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(q);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.), r);
    }
    if q == 0. { 0. } else { q * r }
}
fn length(v: V) -> f32 {
    sqrt(dot(v, v))
}
fn horizontal_length(mut v: V) -> f32 {
    v[1] = 0.;
    length(v)
}
fn unit(v: V) -> V {
    let n = length(v);
    if n > f32::from_bits(0x358637bd) {
        scale(v, 1. / n)
    } else {
        [0.; 4]
    }
}
