//! Original S3 82D4D150/82E09D28; S2 82D97D68 lacks S3 eligibility tests.
//! PC reciprocal estimates retain native refinement steps, not Xenon bit parity.
use crate::player::offboard::grab_scene::{Record, closest_point};
type Vector = [f32; 4];

fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: Vector, s: f32) -> Vector {
    a.map(|x| x * s)
}
fn inverse_length(squared: f32) -> f32 {
    let mut r = crate::physics::reciprocal_sqrt::estimate(squared);
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-squared).mul_add(r * r, 1.), r);
    }
    r
}
fn horizontal(mut v: Vector) -> Vector {
    v[1] = 0.;
    let squared = dot(v, v);
    let r = inverse_length(squared);
    let length = if squared == 0. { 0. } else { squared * r };
    //82F826F8 initializes830BD350 from82181A88, a LENGTH threshold.
    if length > f32::from_bits(0x358637bd) {
        scale(v, r)
    } else {
        [0.; 4]
    }
}

///Select the nearest record FIRST, then return that record's eligibility.
///An ineligible nearer record suppresses a farther eligible record.
pub fn best_spline(records: &[Record], position: Vector) -> Option<Record> {
    let mut distance = f32::from_bits(0x47c34ff3); //822F8FF0.
    let mut best = None;
    for record in records {
        let endpoints = record.endpoints();
        let delta = sub(closest_point(position, endpoints), position);
        let direction = horizontal(sub(endpoints[0], endpoints[1]));
        let approach = horizontal(delta);
        let eligible = delta[1] > f32::from_bits(0xbf19999a)
            && delta[1] < f32::from_bits(0x3f19999a)
            && dot(direction, approach).abs() < f32::from_bits(0x3f4ccccd);
        let candidate_distance = dot(delta, delta);
        //82D4D39C is bge: unordered does not take the skip branch.
        if !(candidate_distance >= distance) {
            distance = candidate_distance;
            best = eligible.then(|| record.clone());
        }
    }
    best
}
