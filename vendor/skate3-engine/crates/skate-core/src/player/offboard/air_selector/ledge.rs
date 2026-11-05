//! Air-only ledge correction82D6D4D0, filtering82C1F310, six-line82C20728.
//! Candidate0 only. Closest-point/plane formulas read from82E09D28/09DF0.
use super::{Candidate, Context, DT, Vector, math::*};
use crate::{
    air::trajectory::{Prediction, Trajectory},
    math::Vector3,
    player::offboard::ground_query::{Edge, EdgeSearch, Frame, Line, LineHit, QueryContext},
};
fn lanes(v: Vector3) -> Vector {
    [v.x, v.y, v.z, 0.]
}
fn vec3(v: Vector) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
pub fn search(
    candidate: Candidate,
    prediction: Prediction,
    context: Context,
) -> Option<EdgeSearch> {
    if !prediction.result.valid() || prediction.result.contact_frame < 16 {
        return None;
    }
    let collision = prediction.collision_position();
    let (apex, _) = candidate.trajectory.highest_position();
    Some(EdgeSearch {
        min: vec3(std::array::from_fn(|i| collision[i].min(apex[i]) - 2.)),
        max: vec3(std::array::from_fn(|i| collision[i].max(apex[i]) + 2.)),
        frame: Frame {
            position: vec3(candidate.trajectory.position),
            ..Frame::IDENTITY
        },
        context: QueryContext {
            selection_flags_2948: context.selection_flags_2948,
            matching_id_2952: context.matching_group_2952,
        },
        narrow_forward: false,
    })
}
///82C1F310 removes a parallel edge only with native separation, overlap,
/// reference-distance and oriented-height tests. Retains provider order.
pub fn filter_edges(edges: &[Edge], reference: Vector) -> Vec<Edge> {
    let directions: Vec<_> = edges
        .iter()
        .map(|e| normalize(sub(lanes(e.end), lanes(e.start))))
        .collect();
    edges
        .iter()
        .enumerate()
        .filter_map(|(i, &edge)| {
            let start = lanes(edge.start);
            let midpoint = scale(add(start, lanes(edge.end)), 0.5);
            for (j, other) in edges.iter().enumerate() {
                if i == j {
                    continue;
                }
                let parallel = cross(directions[i], directions[j]);
                if !(dot(parallel, parallel) < 0.0001) {
                    continue;
                }
                let delta = sub(lanes(other.start), start);
                let off = sub(delta, scale(directions[j], dot(directions[j], delta)));
                let distance = dot(off, off);
                if !(distance > 0.0001 && distance < 0.010000001) {
                    continue;
                }
                let from_mid = sub(lanes(other.start), midpoint);
                if !(dot(from_mid, sub(lanes(other.end), midpoint)) < 0.) {
                    continue;
                }
                let other_point = add(
                    midpoint,
                    sub(from_mid, scale(directions[j], dot(directions[j], from_mid))),
                );
                let a = sub(midpoint, reference);
                let b = sub(other_point, reference);
                let normal = normalize_or(cross(directions[i], cross(UP, directions[i])), [0.; 4]);
                let height = dot(delta, normal);
                if (height > -0.02 && dot(a, a) > dot(b, b)) || height > 0.02 {
                    return None;
                }
            }
            Some(edge)
        })
        .take(40)
        .collect()
}
///82D60C80/B98: exact discriminant branches, larger root for two roots.
/// No generic epsilon-linear fallback, positive clamp, or duration rejection.
pub fn plane_time(t: Trajectory, point: Vector, normal: Vector) -> Option<f32> {
    let a = dot(normal, scale(t.acceleration, 0.5));
    let b = dot(normal, t.velocity);
    let c = dot(normal, t.position) - dot(point, normal);
    let discriminant = b * b - (4. * a) * c;
    if discriminant < 0. {
        return None;
    }
    let denominator = reciprocal(2. * a);
    if discriminant > 0. {
        let root = discriminant
            * crate::physics::board_motion_output::inverse_length_squared(discriminant, 2);
        Some((denominator * (-b + root)).max(denominator * (-b - root)))
    } else {
        let time = denominator * (-b);
        (time > 0.).then_some(time)
    }
}
fn closest(point: Vector, edge: Edge) -> Vector {
    let start = lanes(edge.start);
    let mut delta = sub(lanes(edge.end), start);
    let n = length(delta);
    if n > f32::from_bits(0x37800000) {
        delta = scale(delta, reciprocal(n));
    }
    madd(delta, dot(delta, sub(point, start)).min(n).max(0.), start)
}
fn angle(new: Vector, old: Vector, up: Vector) -> f32 {
    if !(dot(new, new) * dot(old, old) > f32::from_bits(0x37800000)) {
        return 0.;
    }
    let a = sub(new, scale(up, dot(up, new)));
    let b = sub(old, scale(up, dot(up, old)));
    let (aa, bb) = (dot(a, a), dot(b, b));
    if !(aa > 0.0001 && bb > 0.0001) {
        return 0.;
    }
    //8296EC98 uses ONE refinement, not the two-refinement generic normalize.
    let a = scale(
        a,
        crate::physics::board_motion_output::inverse_length_squared(aa, 1),
    );
    let b = scale(
        b,
        crate::physics::board_motion_output::inverse_length_squared(bb, 1),
    );
    let angle = crate::trigonometry::acos(dot(a, b).max(-1.).min(1.));
    let angle = if dot(cross(a, b), up) < 0. {
        std::f32::consts::TAU - angle
    } else {
        angle
    };
    wrap_angle(angle).abs()
}
#[derive(Clone, Copy, Debug)]
pub struct Adjustment {
    pub edge: Edge,
    pub point: Vector,
    pub lowered_trajectory: Trajectory,
    pub landing_frame: i32,
    pub radius: f32,
}
impl Adjustment {
    pub fn apply(self, candidate: &mut Candidate) {
        candidate.trajectory = self.lowered_trajectory;
        candidate.trajectory.position =
            add(candidate.trajectory.position, [0., self.radius, 0., 0.]);
        candidate.normal_64 = normalize_or(
            cross(
                sub(lanes(self.edge.end), lanes(self.edge.start)),
                cross(UP, sub(lanes(self.edge.end), lanes(self.edge.start))),
            ),
            UP,
        );
        candidate.contact_position_96 = self.point;
        candidate.landing_frame_116 = self.landing_frame;
        candidate.contact_velocity_80 = self
            .lowered_trajectory
            .velocity_at(self.landing_frame as f32 * DT);
        candidate.special_121 = true;
    }
}
pub fn choose(
    candidate: Candidate,
    prediction: Prediction,
    context: Context,
    radius: f32,
    edges: &[Edge],
) -> Option<Adjustment> {
    if !prediction.result.valid() || prediction.result.contact_frame < 16 {
        return None;
    }
    let t = candidate.trajectory;
    let mut lowered = t;
    lowered.position = add(t.position, [0., -radius, 0., 0.]);
    let mut best: Option<Adjustment> = None;
    for &edge in edges {
        let delta = sub(lanes(edge.end), lanes(edge.start));
        let normal = normalize_or(cross(delta, cross(UP, delta)), UP);
        let Some(time) = plane_time(t, lanes(edge.start), normal) else {
            continue;
        };
        let intersection = t.position_at(time);
        let point = closest(intersection, edge);
        if !(normal[1] > 0.71) {
            continue;
        }
        let horizontal = flatten(sub(point, lowered.position));
        let distance = length(horizontal);
        if !(distance > 0.3) {
            continue;
        }
        let direction = scale(horizontal, 1. / distance);
        if !(dot(
            direction,
            normalize_or(flatten(context.forward_224), [0.; 4]),
        ) > 0.71)
        {
            continue;
        }
        let frame = (time * 59.999996) as i32;
        let mut adjusted = lowered;
        //Original delta uses the UNLOWERED continuous intersection, not a
        //rounded frame sample of the lowered working trajectory (incoming).
        adjust(&mut adjusted, frame, sub(point, intersection), 10.);
        let velocity_delta = sub(adjusted.velocity, t.velocity);
        let height = point[1] - prediction.result.contact_position[1];
        if !(height > -0.05 && dot(velocity_delta, velocity_delta) < 2.25) {
            continue;
        }
        let near_height = height < 0.05;
        let maximum = if near_height { 10. } else { 40. };
        if near_height && !(dot(direction, normalize_or(flatten(delta), [0.; 4])) >= 0.71) {
            continue;
        }
        if !(angle(adjusted.velocity, t.velocity, context.up_544)
            < maximum * f32::from_bits(0x3c8efa35))
        {
            continue;
        }
        if best.as_ref().is_some_and(|old| old.point[1] > point[1]) {
            continue;
        }
        best = Some(Adjustment {
            edge,
            point,
            lowered_trajectory: adjusted,
            landing_frame: frame,
            radius,
        });
    }
    best
}
///82C20530 clears byte64: Air does not append Ground's downward seventh line.
pub fn lines(adjustment: Adjustment, half_wheelbase: f32) -> Option<[Line; 6]> {
    let edge = adjustment.edge;
    let delta = sub(lanes(edge.end), lanes(edge.start));
    let raw_up = cross(delta, cross(UP, delta));
    if 0.00001 > length(raw_up) {
        return None;
    }
    let up = normalize_or(raw_up, raw_up);
    let tangent = normalize_or(delta, delta);
    let start = lanes(edge.start);
    let projection = madd(tangent, dot(sub(adjustment.point, start), tangent), start);
    let center = sub(adjustment.point, sub(adjustment.point, projection));
    let side = cross(up, tangent);
    let near = scale(side, 0.09);
    let far = scale(side, half_wheelbase);
    let short_up = scale(up, 0.04);
    let far_up = scale(up, half_wheelbase * f32::from_bits(0x3f87ae14));
    let line = |c, d, radius| Line {
        start: vec3(add(c, d)),
        end: vec3(sub(c, d)),
        radius,
    };
    Some([
        line(add(center, near), short_up, 0.),
        line(sub(center, near), short_up, 0.),
        line(add(center, far), far_up, 0.),
        line(sub(center, far), far_up, 0.),
        line(center, short_up, 0.),
        line(add(center, scale(up, 0.09)), near, 0.001),
    ])
}
///82C20C08: return1 after consuming a submitted batch, even with six misses.
///The general grind-classification output is temporary and discarded by Air.
pub fn consume_lines(hits: &[Option<LineHit>]) -> Result<(), &'static str> {
    if hits.len() != 6 {
        Err("BipedAir ledge completion requires exactly six lines")
    } else {
        Ok(())
    }
}
