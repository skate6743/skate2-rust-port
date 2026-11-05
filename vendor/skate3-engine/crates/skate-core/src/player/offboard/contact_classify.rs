//! Candidate generation82D837C8. Uses the recovered sorted/reduced segments;
//! final candidate selection and contact packet publication happen afterward.
use super::{
    contact_candidates,
    contact_geometry::{slope_between, slope_limit},
    contact_packet::Packet,
    contact_queries::{Input, V},
    contact_segments::{Candidate, Kind, Segments, length_inverse},
};
use crate::physics::native_arithmetic::dot3;
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn height(input: Input, p: V) -> f32 {
    dot3(input.surface_up, sub(p, input.position))
}
fn forward(input: Input, p: V) -> f32 {
    dot3(input.surface_forward, sub(p, input.position))
}
pub fn candidates(
    input: Input,
    packet: &Packet,
    segments: &Segments,
    forward_limit: f32,
    previous_flags: u32,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    let supported = packet.flags & 1 != 0;
    if supported {
        let up_speed = dot3(input.surface_up, input.velocity);
        let planar = std::array::from_fn(|i| input.velocity[i] - input.surface_up[i] * up_speed);
        if !(length_inverse(planar).0 >= f32::from_bits(0x3c23_d70a)) {
            let a = input.surface_right;
            let b = packet.normal;
            let tangent = [
                (-a[2]).mul_add(b[1], a[1] * b[2]),
                (-a[0]).mul_add(b[2], a[2] * b[0]),
                (-a[1]).mul_add(b[0], a[0] * b[1]),
                (-a[3]).mul_add(b[3], a[3] * b[3]),
            ];
            out.push(Candidate::new(
                input,
                segments,
                packet.position,
                packet.normal,
                tangent,
                None,
                0,
                0,
            ));
            return out;
        }
    }
    let count = segments.last_surface_end;
    if count == 0 {
        return out;
    }
    let mut occluded = false;
    let mut passed_drop = false;
    let mut highest: Option<usize> = None;
    for i in 0..count {
        let s = segments.items[i];
        let normal = if i + 1 < count {
            let next = segments.items[i + 1];
            if s.kind != Kind::Surface && next.kind != Kind::Surface {
                input.surface_up
            } else {
                next.normal
            }
        } else {
            s.normal
        };
        let was_drop = passed_drop;
        if !passed_drop {
            let previous = highest.map_or(input.position, |j| segments.items[j].end);
            if dot3(sub(s.end, previous), input.surface_up) > 0. {
                highest = Some(i);
            }
        }
        if s.kind == Kind::Rising && s.length > f32::from_bits(0x3d4c_cccd) {
            let tall = s.length > f32::from_bits(0x3ecc_cccd);
            let mut bridged = false;
            let mut blocked = false;
            let mut slope = slope_limit(input, s.length, true);
            let mut j = i + 1;
            while j < count {
                let next = segments.items[j];
                if !blocked && next.kind == Kind::Falling {
                    blocked =
                        forward(input, next.start) < forward_limit - f32::from_bits(0x3eb3_3333);
                }
                if next.kind == Kind::Surface
                    || (next.kind == Kind::Rising && next.length < f32::from_bits(0x3d4c_cccd))
                {
                    if !blocked {
                        blocked = dot3(next.direction, input.surface_forward)
                            > f32::from_bits(0x3f34_fdf4)
                            && forward(input, next.start)
                                < forward_limit - f32::from_bits(0x3eb3_3333);
                    }
                    j += 1;
                    continue;
                }
                break;
            }
            if j < count && segments.items[j].kind == Kind::Rising {
                let between = slope_between(input, s.end, segments.items[j].end);
                if between > slope_limit(input, s.length, true) * 0.5 {
                    slope = between;
                    bridged = true;
                    if !occluded
                        && slope_between(input, input.position, segments.items[j].end)
                            < slope_limit(input, s.length, true) * 0.5
                    {
                        bridged = false;
                    }
                }
            }
            if !occluded {
                occluded = blocked;
            }
            if !blocked {
                continue;
            }
            if let Some(c) =
                contact_candidates::rising(input, segments, i, slope, if tall { 128 } else { 0 })
            {
                out.push(c);
            }
            out.push(Candidate::from_point(
                input,
                segments,
                s.end,
                normal,
                Some(i),
                64 | if bridged || tall { 256 } else { 0 },
                if bridged { 2 } else { 3 },
            ));
        } else if s.kind == Kind::Falling && s.length > f32::from_bits(0x3d4c_cccd) {
            let mut bridged = false;
            let tall = s.length > f32::from_bits(0x3ecc_cccd);
            let mut slope = -slope_limit(input, s.length, false);
            let mut j = i + 1;
            while j < count {
                let next = segments.items[j];
                if next.kind == Kind::Surface
                    || (next.kind == Kind::Falling && next.length < f32::from_bits(0x3d4c_cccd))
                {
                    j += 1;
                    continue;
                }
                break;
            }
            if j < count && segments.items[j].kind == Kind::Falling {
                let between = slope_between(input, s.start, segments.items[j].start);
                if between < slope_limit(input, s.length, false) * -0.5 {
                    bridged = true;
                    slope = between;
                }
            }
            let mut reject = !was_drop
                && !occluded
                && highest.is_some_and(|j| {
                    height(input, segments.items[j].end) > f32::from_bits(0x3ca3_d70a)
                });
            if i == 1 && segments.items[0].kind == Kind::Surface {
                reject = false;
            }
            passed_drop = true;
            if reject {
                continue;
            }
            if !bridged {
                if forward(input, s.start) > 0.75 {
                    continue;
                }
                if let Some(c) = contact_candidates::falling(
                    input,
                    segments,
                    Some(i),
                    slope,
                    if tall { 256 } else { 0 },
                    4,
                    forward_limit,
                ) {
                    out.push(c);
                }
            }
            out.push(Candidate::from_point(
                input,
                segments,
                s.start,
                normal,
                Some(i),
                64 | if bridged || tall { 128 } else { 0 },
                if bridged { 5 } else { 6 },
            ));
        }
    }
    if supported && input.position[1] - packet.position[1] > f32::from_bits(0x3c23_d70a) {
        let mut flags = previous_flags & 0x140;
        if !(input.position[1] - packet.position[1] < f32::from_bits(0x3ecc_cccd)) {
            flags |= 64;
        }
        let empty = out.is_empty();
        if empty || !passed_drop {
            if let Some(c) = contact_candidates::falling(
                input,
                segments,
                None,
                -slope_limit(input, f32::from_bits(0x3f4c_cccd), false),
                flags,
                if empty { 7 } else { 8 },
                forward_limit,
            ) {
                out.push(c);
            }
        }
    } else if out.is_empty() {
        if let Some(c) = contact_candidates::distance_fallback(
            input,
            segments,
            length_inverse(input.velocity).0 * f32::from_bits(0x3c88_8889),
        ) {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::offboard::contact_records::Record;
    fn input(speed: f32) -> Input {
        Input {
            position: [0.; 4],
            surface_forward: [0., 0., 1., 0.],
            surface_up: [0., 1., 0., 0.],
            surface_right: [1., 0., 0., 0.],
            velocity: [0., 0., speed, 0.],
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
    fn stationary_support_and_moving_flat_ground_take_distinct_native_paths() {
        let packet = Packet {
            flags: 1,
            ..Packet::default()
        };
        let stationary = candidates(input(0.), &packet, &Segments::default(), 100., 0);
        assert_eq!(stationary.len(), 1);
        assert_eq!(stationary[0].kind, 0);
        let mut segments = Segments::default();
        segments.rebuild(input(3.), &[r(0., 0.), r(0., 2.)]);
        let moving = candidates(input(3.), &packet, &segments, 100., 0);
        assert_eq!(moving.len(), 1);
        assert_eq!(moving[0].kind, 9);
        assert!((moving[0].position[2] - 0.05).abs() < 1e-6);
    }
    #[test]
    fn step_up_with_forward_clearance_produces_approach_and_top_candidates() {
        let packet = Packet {
            flags: 1,
            ..Packet::default()
        };
        let mut segments = Segments::default();
        segments.rebuild(input(3.), &[r(0., 0.), r(0., 0.5), r(0.3, 0.5), r(0.3, 2.)]);
        let candidates = candidates(input(3.), &packet, &segments, 100., 0);
        assert_eq!(
            candidates.iter().map(|c| c.kind).collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(candidates[1].flags, 64);
        assert_eq!(candidates[1].position, [0., 0.3, 0.5, 0.]);
    }
}
