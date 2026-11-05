//! Profile cleanup82D830A8 and segment construction82D83438.
use super::analyzer_math::{normalize, reciprocal};
use super::{ContactSample, Input, Vector, cross, dot, length, scale, sub};
#[derive(Clone, Copy, Debug)]
pub(super) struct Segment {
    pub kind: u32,
    pub start: Vector,
    pub end: Vector,
    pub direction: Vector,
    pub normal: Vector,
    pub length: f32,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Profile {
    pub segments: Vec<Segment>,
    pub last_ground: usize,
}
pub(super) fn simplify(input: Input, points: &mut Vec<ContactSample>) -> (f32, usize) {
    let mut obstruction = 1e10;
    if points.len() < 2 {
        return (obstruction, points.len());
    }
    let mut count = points.len();
    let removed = points.last().unwrap().sort_distance + 1.;
    for i in 1..points.len() - 1 {
        if points[i].flags & 8 != 0 {
            count -= points.len() - 1 - i;
            obstruction = dot(sub(points[i].position, input.position), input.forward);
            break;
        }
        let incoming = [
            points[i].forward_distance - points[i - 1].forward_distance,
            points[i].height - points[i - 1].height,
            0.,
            0.,
        ];
        let outgoing = [
            points[i + 1].forward_distance - points[i].forward_distance,
            points[i + 1].height - points[i].height,
            0.,
            0.,
        ];
        let a = length(incoming);
        let b = length(outgoing);
        let discard = a < 0.02
            || b < 0.02
            || (!(dot(incoming, outgoing) < 0. && dot(incoming, input.up) >= 0.)
                && length(cross(
                    scale(incoming, reciprocal(a)),
                    scale(outgoing, reciprocal(b)),
                )) < 0.05);
        if discard {
            points[i].sort_distance = removed;
            count -= 1;
        }
    }
    sort(points);
    // Native count14928 shrinks, but backing records survive for the later
    // publication scan bounded by retained original count14932.
    (obstruction, count)
}
pub(super) fn sort(points: &mut [ContactSample]) {
    points.sort_by(|a, b| {
        a.sort_distance
            .partial_cmp(&b.sort_distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
pub(super) fn build(input: Input, points: &[ContactSample]) -> Profile {
    let mut out = Profile::default();
    for pair in points.windows(2) {
        let start = pair[0].position;
        let end = pair[1].position;
        let delta = sub(end, start);
        let distance = length(delta);
        if distance < 0.0001 {
            continue;
        }
        let direction = scale(delta, reciprocal(distance));
        let kind = if dot(input.forward, direction).abs() > 0.5 {
            0
        } else if dot(input.up, direction) >= 0. {
            1
        } else {
            2
        };
        if kind != 0 && out.segments.last().is_some_and(|last| last.kind == kind) {
            let last = out.segments.last_mut().unwrap();
            last.end = end;
            let delta = sub(end, last.start);
            last.length = length(delta);
            last.direction = scale(delta, reciprocal(last.length));
            last.normal = normalize(cross(delta, input.right), input.up);
        } else {
            out.segments.push(Segment {
                kind,
                start,
                end,
                direction,
                normal: normalize(cross(delta, input.right), input.up),
                length: distance,
            });
            if kind == 0 {
                out.last_ground = out.segments.len();
            }
        }
    }
    out
}
