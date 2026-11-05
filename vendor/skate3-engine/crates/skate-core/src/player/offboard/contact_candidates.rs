//! Native candidate searches82D84D38/85070 and distance fallback82D84B98.
use super::{
    contact_geometry,
    contact_queries::{Input, V},
    contact_segments::{Candidate, Kind, Segments},
};
use crate::physics::native_arithmetic::dot3;
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn project(input: Input, p: V) -> [f32; 2] {
    let delta = sub(p, input.position);
    [
        dot3(input.surface_forward, delta),
        dot3(input.surface_up, delta),
    ]
}
fn next_normal(input: Input, segments: &Segments, i: usize) -> V {
    if i + 1 == segments.last_surface_end {
        input.surface_up
    } else {
        segments.items[i + 1].normal
    }
}
pub fn rising(
    input: Input,
    segments: &Segments,
    index: usize,
    slope: f32,
    flags: u32,
) -> Option<Candidate> {
    let anchor = project(input, segments.items[index].end);
    let end = [-1., (-1. - anchor[0]).mul_add(slope, anchor[1])];
    for i in (0..index).rev() {
        let s = segments.items[i];
        let fraction = contact_geometry::intersect(
            project(input, s.start),
            project(input, s.end),
            anchor,
            end,
        );
        if fraction < 0. {
            if i == 0 {
                return Some(Candidate::from_point(
                    input,
                    segments,
                    input.position,
                    input.surface_up,
                    Some(index),
                    flags,
                    1,
                ));
            }
            continue;
        }
        if !(fraction <= 1.) {
            continue;
        }
        if s.kind != Kind::Surface {
            if dot3(input.surface_up, sub(s.start, input.position)) > 0.75 {
                return None;
            }
            return Some(Candidate::from_point(
                input,
                segments,
                s.start,
                next_normal(input, segments, i),
                Some(index),
                flags,
                1,
            ));
        }
        let distance = s.length * fraction;
        let position = std::array::from_fn(|k| s.direction[k].mul_add(distance, s.start[k]));
        if dot3(input.surface_up, sub(position, input.position)) > 0.75 {
            return None;
        }
        return Some(Candidate::from_point(
            input,
            segments,
            position,
            s.normal,
            Some(index),
            flags,
            1,
        ));
    }
    None
}
pub fn falling(
    input: Input,
    segments: &Segments,
    index: Option<usize>,
    slope: f32,
    flags: u32,
    kind: u32,
    forward_limit: f32,
) -> Option<Candidate> {
    let anchor = project(
        input,
        index.map_or(input.position, |i| segments.items[i].start),
    );
    let x = f32::from_bits(0x4033_3333);
    let end = [x, (x - anchor[0]).mul_add(slope, anchor[1])];
    for i in index.map_or(0, |i| i + 1)..segments.last_surface_end {
        let s = segments.items[i];
        let fraction = contact_geometry::intersect(
            project(input, s.start),
            project(input, s.end),
            anchor,
            end,
        );
        if fraction < 0. || !(fraction <= 1.) {
            continue;
        }
        let (position, normal) = if s.kind != Kind::Surface {
            if dot3(input.surface_forward, sub(s.end, input.position))
                > forward_limit - f32::from_bits(0x3eb3_3333)
            {
                return None;
            }
            (s.end, next_normal(input, segments, i))
        } else {
            let distance = s.length * fraction;
            (
                std::array::from_fn(|k| s.direction[k].mul_add(distance, s.start[k])),
                s.normal,
            )
        };
        return Some(Candidate::from_point(
            input, segments, position, normal, index, flags, kind,
        ));
    }
    None
}
pub fn distance_fallback(
    input: Input,
    segments: &Segments,
    mut distance: f32,
) -> Option<Candidate> {
    for i in 0..segments.last_surface_end {
        let s = segments.items[i];
        if s.kind == Kind::Falling {
            distance += s.length / contact_geometry::slope_limit(input, s.length, false);
            continue;
        }
        distance -= s.length;
        if !(distance <= 0.) {
            continue;
        }
        let (position, normal) = if s.kind == Kind::Rising {
            (s.end, next_normal(input, segments, i))
        } else {
            (
                std::array::from_fn(|k| s.direction[k].mul_add(distance, s.end[k])),
                s.normal,
            )
        };
        return Some(Candidate::from_point(
            input,
            segments,
            position,
            normal,
            Some(i),
            0,
            9,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::offboard::contact_records::Record;
    fn input() -> Input {
        Input {
            position: [0.; 4],
            surface_forward: [0., 0., 1., 0.],
            surface_up: [0., 1., 0., 0.],
            surface_right: [1., 0., 0., 0.],
            velocity: [0., 0., 2., 0.],
            animation_up: [0., 1., 0., 0.],
            animation_right: [1., 0., 0., 0.],
        }
    }
    fn r(y: f32, z: f32) -> Record {
        Record {
            position: [0., y, z, 0.],
            normal: [0., 1., 0., 0.],
            coordinates: [z, y, z, z],
            flags: 0,
            distance: z,
        }
    }
    #[test]
    fn searches_intersect_actual_surface_segments() {
        let mut segments = Segments::default();
        segments.rebuild(input(), &[r(0., 0.), r(0., 1.), r(0.5, 1.), r(0.5, 2.)]);
        let candidate = rising(input(), &segments, 1, 1., 128).unwrap();
        assert_eq!(candidate.position, [0., 0., 0.5, 0.]);
        assert_eq!(candidate.flags, 128);
        assert_eq!(candidate.segment_index, Some(1));
        let candidate = distance_fallback(input(), &segments, 0.25).unwrap();
        assert_eq!(candidate.position, [0., 0., 0.25, 0.]);
        assert_eq!(candidate.kind, 9);
        segments.rebuild(input(), &[r(0., 0.), r(0., 1.), r(-0.5, 1.), r(-0.5, 2.)]);
        let candidate = falling(input(), &segments, Some(1), -1., 0, 4, 100.).unwrap();
        assert_eq!(candidate.position, [0., -0.5, 1.5, 0.]);
        assert_eq!(candidate.kind, 4);
    }
}
