//! TU3 scoring82D68FE8/82D68AF0/82D69790 for a world with no grind edges.
//! That topology is supplied by the authored BoardWorld, not inferred from a
//! failed collision. Every wall probe still queries real world geometry.
use super::{
    Prediction, SelectorInput, SelectorSettings, SurfaceHit, launch::adjust_trajectory, math::*,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Candidate {
    pub prediction: Prediction,
    pub start_velocity: Vector,     //candidate64+0, before wall adjustment
    pub normal: Vector,             //+16
    pub collision_velocity: Vector, //+32, before wall adjustment
    pub collision_position: Vector, //+48, before wall adjustment
    pub score: f32,
    pub wall_score: f32,
    pub wall_ride: bool,
    pub grind: Option<super::grind::GrindTarget>,
}
pub(super) fn score(
    candidates: &mut [Candidate],
    pass: u16,
    adjusted_on_vert: bool,
    input: SelectorInput,
    s: &SelectorSettings,
    mut grind: impl FnMut(&mut Prediction, bool) -> Result<super::grind::GrindEvaluation, String>,
    mut line: impl FnMut(Vector, Vector, f32) -> Result<Option<SurfaceHit>, String>,
) -> Result<bool, String> {
    let mut all_miss = true; //selector9662, consumed by PhysicsAir COM latch
    let mut highest_wall = -1_000_000.0;
    for (index, c) in candidates.iter_mut().enumerate() {
        c.score = 0.0;
        c.wall_score = 0.0;
        c.wall_ride = false;
        c.grind = None;
        if !c.prediction.result.valid() {
            continue;
        }
        all_miss = false;
        if c.prediction.result.contact_frame < s.minimum_trajectory_frames {
            continue;
        }
        let middle = index == 0 && pass == 1;
        let middle_bonus = if middle && !adjusted_on_vert {
            s.score_middle_bonus
        } else {
            0.0
        };
        c.collision_position = c.prediction.collision_position();
        c.normal = c.prediction.result.suggested_normal();
        c.prediction.result.contact_normal = c.normal;
        c.collision_velocity = c.prediction.collision_velocity();
        let force_dot = dot(c.normal, c.collision_velocity);
        let sideways = length(cross(c.normal, normalize(c.collision_velocity)));
        let scalar = s.landing_force_scalar.evaluate(force_dot.abs());
        let force_score = (force_dot * s.score_landing_force) * scalar;
        let direction_score = (sideways * s.score_landing_direction) * scalar;

        let evaluation = grind(
            &mut c.prediction,
            middle
                && !(input.flags_2472 & 0x2000_0000 != 0
                    && input.offboard_flags_1776 & 0x0400_0000 != 0
                    && input.offboard_flags_1776 & 0x0800_0000 == 0),
        )?;
        c.grind = evaluation.target;
        let mut grind_score = if input.flags_2476 & 0x0200_0000 != 0 {
            0.0
        } else if evaluation.score != 0.0 {
            evaluation.score
        } else if input.grind_lock_distance > 0.5 || !middle {
            s.grind_penalty_vs_distance
                .evaluate(evaluation.penalty_input())
        } else {
            0.0
        };
        if grind_score <= 0.0 {
            c.wall_score = wall_score(
                &mut c.prediction,
                grind_score < 0.0,
                s,
                &mut highest_wall,
                &mut line,
            )?;
            c.wall_ride = c.wall_score > 0.0;
            if c.wall_score < 0.0 {
                c.prediction.result.contact_normal = UP;
            }
        }
        let mut surface_score = 0.0;
        if grind_score <= 1.0 {
            match (c.prediction.result.surface >> 7) & 31 {
                6 => {
                    surface_score = s.surface_unrideable_score;
                    c.prediction.result.contact_normal = input.reference_up;
                }
                7 => {
                    surface_score = s.surface_dont_align_score;
                    c.prediction.result.contact_normal = input.reference_up;
                }
                8 => {
                    surface_score = s.surface_dont_align_score;
                }
                _ => {}
            }
        }
        c.normal = c.prediction.result.contact_normal;
        let mut transition = 0.0;
        if adjusted_on_vert {
            if input.directional_input < 0.5 && grind_score > 0.0 {
                c.grind = None;
                grind_score = s.grind_penalty_vs_distance.evaluate(0.0);
            }
            let sign = if input.directional_input >= 0.5 {
                1.0
            } else {
                -1.0
            };
            let mut heading = input.heading_direction;
            heading[1] = 0.0;
            let since_apex = (c.prediction.result.contact_time - apex_time(c.prediction)).abs();
            let penalty = if since_apex >= s.minimum_time_after_apex {
                0.0
            } else {
                -500.0
            };
            let coefficient = ((1.0 - input.ground_normal[1].abs()) * sign) * s.score_transition;
            transition = dot(c.normal, normalize(heading)) * coefficient + penalty;
        }
        c.score = s
            .landing_time_bonus
            .evaluate(c.prediction.result.contact_time)
            + (((((direction_score + 1.0) + force_score) + middle_bonus) + grind_score)
                + transition);
        c.score += surface_score;
    }
    //AddWallRideScores runs after ALL candidates, using the final highest Y.
    for c in candidates {
        if c.prediction.result.valid()
            && c.prediction.result.contact_frame >= s.minimum_trajectory_frames
        {
            if c.wall_score == 0.0 && c.prediction.collision_position()[1] > highest_wall {
                c.wall_score = 2000.0;
            }
            c.score += c.wall_score;
        }
    }
    Ok(all_miss)
}

fn apex_time(p: Prediction) -> f32 {
    let v = p.request.trajectory.velocity[1];
    let g = p.request.trajectory.acceleration[1];
    if v < 0.0 || g >= 0.0 {
        0.0
    } else {
        v * reciprocal(-g)
    }
}
fn wall_score(
    p: &mut Prediction,
    reject_grind: bool,
    s: &SelectorSettings,
    highest: &mut f32,
    line: &mut impl FnMut(Vector, Vector, f32) -> Result<Option<SurfaceHit>, String>,
) -> Result<f32, String> {
    let normal = p.result.contact_normal;
    if !(normal[1].abs() < 0.5) {
        return Ok(0.0);
    }
    let start = madd(normal, s.wall_ride_test_distance, p.result.contact_position);
    let end = add(start, [0.0, -10.0, 0.0, 0.0]);
    let Some(hit) = line(start, end, 0.0)? else {
        return Ok(0.0);
    };
    if !(dot(hit.normal, normal) < s.wall_ride_normal_dot_limit) {
        return Ok(0.0);
    }
    let height = (hit.position[1] - start[1]).abs();
    if reject_grind {
        return Ok(height * s.wall_ride_height_score - 1000.0);
    }
    let position = p.collision_position();
    let mut delta = sub(p.request.trajectory.position, position);
    let mut slope = delta[1];
    delta[1] = 0.0;
    let horizontal = length(delta);
    if horizontal > f32::from_bits(0x3c23_d70a) {
        slope *= reciprocal(horizontal);
    }
    if slope > s.wall_ride_angle_allow_landing {
        return Ok(height * s.wall_ride_height_score - 1000.0);
    }
    if height < s.wall_ride_minimum_height {
        return Ok(height - 1000.0);
    }
    let boost = s.wall_ride_boost.evaluate(height);
    if position[1] > *highest {
        *highest = position[1];
    }
    adjust_trajectory(
        &mut p.request.trajectory,
        p.result.contact_frame,
        [0.0, boost, 0.0, 0.0],
        s.maximum_trajectory_adjust,
    );
    Ok(height * s.wall_ride_height_score)
}
