//! Retail trajectory/edge qualification (82C1F310, 82E09DF0, 82D6D4D0).
use super::{
    air_launch::{V, cross, dot, length, madd, scale, sub, unit},
    ground_query::Edge,
};
use crate::air::trajectory::Trajectory;
const UP: V = [0., 1., 0., 0.];
fn vector(v: crate::math::Vector3) -> V {
    [v.x, v.y, v.z, 0.]
}
///82C1F310 removes overlapping parallel edges according to height and distance
///from the trajectory origin, preserving enumeration order and the forty cap.
pub fn visible_edges(edges: &[Edge], origin: V) -> Vec<Edge> {
    let directions: Vec<_> = edges
        .iter()
        .map(|e| unit(sub(vector(e.end), vector(e.start)), [0.; 4]))
        .collect();
    edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let a = vector(e.start);
            let b = vector(e.end);
            let center = scale(madd(a, 1., b), 0.5);
            for (j, other) in edges.iter().enumerate() {
                if i == j {
                    continue;
                }
                let parallel = cross(directions[i], directions[j]);
                if dot(parallel, parallel) >= 0.0001 {
                    continue;
                }
                let delta = sub(vector(other.start), a);
                let perpendicular = sub(delta, scale(directions[j], dot(directions[j], delta)));
                let d = dot(perpendicular, perpendicular);
                if d > 0.0001 && d >= f32::from_bits(0x3c23d70b) {
                    continue;
                }
                let start = sub(vector(other.start), center);
                let end = sub(vector(other.end), center);
                if dot(start, end) >= 0. {
                    continue;
                }
                let projected = madd(directions[j], -dot(directions[j], start), start);
                let other_center = madd(projected, 1., center);
                let normal = unit(cross(cross(directions[i], UP), directions[i]), [0.; 4]);
                let height = dot(delta, normal);
                if height > -0.02
                    && (length(sub(center, origin)) > length(sub(other_center, origin))
                        || height > 0.02)
                {
                    return None;
                }
            }
            Some(*e)
        })
        .take(40)
        .collect()
}
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub edge: Edge,
    pub point: V,
    pub arc: Trajectory,
    pub frame: i32,
    pub normal: V,
}
///82E09DF0 solves the plane through the edge and returns the descending root.
pub fn candidate(
    arc: Trajectory,
    edge: Edge,
    radius: f32,
    forward: V,
    up: V,
    impact_y: f32,
) -> Option<Candidate> {
    let a = vector(edge.start);
    let delta = sub(vector(edge.end), a);
    let normal = unit(cross(cross(delta, UP), delta), UP);
    if normal[1] <= 0.71 {
        return None;
    }
    let qa = dot(arc.acceleration, normal) * 0.5;
    let qb = dot(arc.velocity, normal);
    let qc = dot(sub(arc.position, a), normal);
    let time = if qa.abs() < 1e-6 {
        if qb.abs() < 1e-6 {
            return None;
        }
        -qc / qb
    } else {
        let discriminant = qb * qb - 4. * qa * qc;
        if discriminant < 0. {
            return None;
        }
        let root = discriminant.sqrt();
        ((-qb - root) / (2. * qa)).max((-qb + root) / (2. * qa))
    };
    if time <= 0. {
        return None;
    }
    let intersection = arc.position_at(time);
    let segment_length = dot(delta, delta);
    let point = if segment_length > 1e-12 {
        madd(
            delta,
            (dot(sub(intersection, a), delta) / segment_length).clamp(0., 1.),
            a,
        )
    } else {
        a
    };
    let mut adjusted = arc;
    adjusted.position[1] -= radius;
    let mut horizontal = sub(point, adjusted.position);
    horizontal[1] = 0.;
    if length(horizontal) <= 0.3 {
        return None;
    }
    let direction = unit(horizontal, [0.; 4]);
    let mut facing = forward;
    facing[1] = 0.;
    if dot(direction, unit(facing, [0.; 4])) <= 0.71 {
        return None;
    }
    let frame = (time * f32::from_bits(0x426fffff)) as i32;
    if frame > 0 {
        let correction = scale(
            sub(
                point,
                adjusted.position_at(frame as f32 * f32::from_bits(0x3c888889)),
            ),
            1. / (frame as f32 * f32::from_bits(0x3c888889)),
        );
        adjusted.velocity = madd(
            correction,
            if length(correction) > 10. {
                10. / length(correction)
            } else {
                1.
            },
            adjusted.velocity,
        );
    }
    let height = point[1] - impact_y;
    if height <= -0.05 {
        return None;
    }
    let difference = sub(adjusted.velocity, arc.velocity);
    if dot(difference, difference) >= 2.25 {
        return None;
    }
    let near = height < 0.05;
    let mut along = delta;
    along[1] = 0.;
    if near && dot(direction, unit(along, [0.; 4])) < 0.71 {
        return None;
    }
    let old = unit(sub(arc.velocity, scale(up, dot(up, arc.velocity))), [0.; 4]);
    let new = unit(
        sub(adjusted.velocity, scale(up, dot(up, adjusted.velocity))),
        [0.; 4],
    );
    let angle = crate::trigonometry::acos(dot(old, new).clamp(-1., 1.)).abs();
    if angle >= (if near { 10. } else { 40. }) * f32::from_bits(0x3c8efa35) {
        return None;
    }
    adjusted.position[1] += radius;
    Some(Candidate {
        edge,
        point,
        arc: adjusted,
        frame,
        normal,
    })
}
