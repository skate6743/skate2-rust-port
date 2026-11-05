//! Physical Ground entry82D37604..82D378A8 and future-deck82D38630 math.
//! These operate on actual board rates/force records. Skeleton publication is
//! separate because prediction moves its animation target, not the board.
use crate::{
    math::Vector3,
    physics::{
        force_queue::{BoardForceQueue, QueuedPointForce},
        native_arithmetic::reciprocal_estimate,
    },
    riding::ground_correction_math::dot_product,
};

fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn words(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn reciprocal(value: f32) -> f32 {
    let mut r = reciprocal_estimate(value);
    for _ in 0..2 {
        r = r.mul_add((-value).mul_add(r, 1.0), r);
    }
    r
}
/// Projection is applied directly to deck angular velocity, before speed seed.
pub fn entry_angular_velocity(normal: [f32; 4], angular: [f32; 4]) -> Vector3 {
    let speed = dot_product(normal, angular);
    xyz(normal.map(|v| v * speed))
}
/// Board+260 is the smaller absolute longitudinal speed before/after normal
/// rejection. The source fsel retains the raw value on an unordered compare.
pub fn entry_target_speed(velocity: Vector3, normal: [f32; 4], forward: [f32; 4]) -> f32 {
    let v = words(velocity);
    let along_normal = dot_product(v, normal);
    let tangent = core::array::from_fn(|i| v[i] - normal[i] * along_normal);
    let raw = dot_product(v, forward).abs();
    let planar = dot_product(tangent, forward).abs();
    if raw - planar >= 0.0 { planar } else { raw }
}
/// Exact tag19 producer. Gravity and normal velocity are not added here.
pub fn landing_on_deck_force(
    physical_velocity: [f32; 4],
    animation_velocity: [f32; 4],
    axis: [f32; 4],
    mass: f32,
    strength: f32,
    point_y: f32,
) -> QueuedPointForce {
    let delta = core::array::from_fn(|i| animation_velocity[i] - physical_velocity[i]);
    let projected = dot_product(delta, axis);
    let step_inverse = reciprocal(f32::from_bits(0x3c88_8889));
    let force = core::array::from_fn(|i| {
        ((delta[i] - axis[i] * projected) * strength) * mass * step_inverse
    });
    QueuedPointForce {
        tag: 19,
        force_world: xyz(force),
        point_body: Vector3::new(0.0, point_y, 0.0),
    }
}
/// Complete displacement calculation in PredictFutureDeck. Force summation
/// preserves queue order; normal rejection occurs after the first dt/mass.
/// Returns None when the source deliberately preserves both skeleton targets.
pub fn future_deck_displacement(
    forces: &BoardForceQueue,
    mass: f32,
    dt: f32,
    normal: [f32; 4],
    pushing: bool,
    manual_correction: bool,
) -> Option<Vector3> {
    if pushing || manual_correction {
        return None;
    }
    let mut sum = [0.0; 4];
    for force in forces.entries() {
        let v = words(force.force_world);
        for i in 0..4 {
            sum[i] += v[i];
        }
    }
    let scale = reciprocal(mass) * dt;
    let delta = sum.map(|v| v * scale);
    let projection = dot_product(delta, normal);
    Some(xyz(core::array::from_fn(|i| {
        (delta[i] - normal[i] * projection) * dt
    })))
}
