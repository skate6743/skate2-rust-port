//! Candidate completion/scoring82D6D020. Geometry correction is called only
//! for the first valid candidate, in source order before scoring its result.
use super::{
    air_launch::{Launch, V},
    air_queries::Candidate,
};
use crate::air::trajectory::QueryResult;
#[derive(Clone, Debug)]
pub struct Completed {
    pub candidate: Candidate,
    pub result: QueryResult,
    pub normal: V,
    pub impact_position: V,
    pub impact_velocity: V,
    pub landing_frame: i32,
    pub contact: bool,
    pub score: f32,
}
pub fn complete(
    launch: &Launch,
    candidates: &[Candidate],
    results: &[QueryResult],
    processed_up: V,
    mut adjust: impl FnMut(&mut Completed) -> Result<bool, String>,
) -> Result<Vec<Completed>, String> {
    if candidates.len() != results.len() {
        return Err("Biped trajectory completion count mismatch".into());
    }
    let mut output = Vec::with_capacity(candidates.len());
    for (index, (candidate, result)) in candidates.iter().zip(results).enumerate() {
        let mut c = Completed {
            candidate: candidate.clone(),
            result: *result,
            normal: [0., 1., 0., 0.],
            impact_position: [0.; 4],
            impact_velocity: [0.; 4],
            landing_frame: 120,
            contact: false,
            score: 0.,
        };
        if result.valid() {
            c.impact_position = result.contact_position;
            let displacement = sub(c.impact_position, candidate.trajectory.position);
            let normal = result.suggested_normal();
            c.normal = normal;
            c.result.contact_normal = normal;
            c.impact_velocity = candidate
                .trajectory
                .velocity_at(result.contact_frame as f32 * f32::from_bits(0x3c888889));
            c.landing_frame = result.contact_frame;
            c.contact = true;
            let mut adjustment = if index == 0 && adjust(&mut c)? {
                1.
            } else {
                0.
            };
            let time = c.landing_frame as f32 * f32::from_bits(0x3c888889);
            let time_penalty = if time <= 0.5 {
                adjustment = 0.;
                -1. - (0.5 - time) * 2.
            } else {
                0.
            };
            if c.result.surface & 0xf80 == 0x300 {
                c.result.contact_normal = processed_up;
            }
            let normal_penalty = if dot(normal, launch.forward) > 0. {
                0.
            } else {
                let value =
                    f32::from_bits(0x400e38e4) * ((normal[1] - f32::from_bits(0x3ee66666)) * 5.);
                if -value >= 0. { value } else { 0. }
            };
            let along = dot(launch.forward, displacement);
            let mut forward = if 1. - along >= 0. { 1. } else { along };
            let mut height = if displacement[1] >= 0. {
                displacement[1]
            } else {
                0.
            };
            if index >= launch.primary_count as usize {
                if displacement[1] < 0.4 {
                    height -= 10000.;
                } else {
                    height *= 8.;
                    forward = dot(launch.secondary_velocity, displacement);
                }
            }
            c.score = ((((forward + height) + normal_penalty) + if index == 0 { 1. } else { 0. })
                + time_penalty)
                + adjustment;
        }
        output.push(c);
    }
    Ok(output)
}
pub fn selected(candidates: &[Completed]) -> Option<usize> {
    let mut best = f32::from_bits(0xccbebc20);
    let mut selected = if candidates.is_empty() { None } else { Some(0) };
    for (index, c) in candidates.iter().enumerate() {
        if c.score > best {
            best = c.score;
            selected = Some(index);
        }
    }
    selected
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V, b: V) -> f32 {
    crate::physics::native_arithmetic::dot3(a, b)
}
