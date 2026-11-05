//! Contact publication82D84060 and slope history82D848D0.
use super::{
    contact_packet::Packet,
    contact_queries::{Input, V},
    contact_records::Records,
    contact_segments::{Candidate, Kind, Segments, length_inverse},
    contact_simplify::Reduction,
};
use crate::physics::native_arithmetic::dot3;
#[derive(Default)]
pub struct History {
    pub retained_candidate: Option<Candidate>,
    pub stable_kind: u32,
    pub samples: [u32; 3],
    pub cursor: usize,
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn forward(input: Input, p: V) -> f32 {
    dot3(input.surface_forward, sub(p, input.position))
}

pub fn publish(
    input: Input,
    packet: &mut Packet,
    records: &Records,
    segments: &Segments,
    reduction: &Reduction,
    candidates: &mut [Candidate],
    history: &mut History,
) {
    edge_output(input, packet, records, segments, reduction);
    super::contact_sort::sort(candidates);
    let near_distance = length_inverse(input.velocity).0 * f32::from_bits(0x3c88_8889);
    let mut selected = None;
    for (i, c) in candidates.iter().copied().enumerate() {
        let near = i + 1 < candidates.len() && forward(input, c.position) < near_distance;
        if c.kind == 1 {
            let mut interrupted = false;
            if !near {
                for next in &candidates[i + 1..] {
                    if next.minimum_distance > c.maximum_distance {
                        break;
                    }
                    if matches!(next.kind, 4 | 7 | 8) {
                        interrupted = true;
                        break;
                    }
                }
            }
            if near || interrupted {
                selected = Some(
                    candidates[i + 1..]
                        .iter()
                        .find(|next| next.segment_index == c.segment_index)
                        .copied()
                        .unwrap_or(c),
                );
                break;
            }
        }
        if !near {
            selected = Some(c);
            break;
        }
    }
    if let Some(c) = selected {
        history.retained_candidate = Some(c);
        packet.target_position = c.position;
        packet.target_normal = c.normal;
        packet.flags |= c.flags | 2;
        if packet.flags & 1 == 0 {
            packet.flags |= 57;
            packet.position = input.position;
            packet.normal = input.surface_up;
        }
        publish_slope(packet, c.tangent, history);
    } else if reduction.forward_limit < f32::from_bits(0x3e99_999a) {
        packet.flags |= 1024;
    } else {
        packet.flags &= !1;
    }
}

fn edge_output(
    input: Input,
    packet: &mut Packet,
    records: &Records,
    segments: &Segments,
    reduction: &Reduction,
) {
    packet.distance_168 = 0.;
    packet.edge_position = input.position;
    packet.edge_normal = input.surface_up;
    if segments.last_surface_end == 0 {
        return;
    }
    let distance = forward(input, segments.items[segments.last_surface_end - 1].end);
    packet.distance_168 = if distance < f32::from_bits(0x3fd3_3333) {
        distance
    } else {
        f32::from_bits(0x5015_02f9)
    };
    if let Some(s) = segments
        .items
        .iter()
        .find(|s| s.kind != Kind::Surface && s.length > f32::from_bits(0x3d4c_cccd))
    {
        packet.distance_168 = forward(input, s.start);
    }
    if packet.flags & 1 != 0
        && input.surface_forward[1] > f32::from_bits(0x3e4c_cccd)
        && f32::from_bits(0x3f33_3333) > packet.normal[1]
    {
        let condition = |r: &&super::contact_records::Record| {
            r.normal[1] > f32::from_bits(0x3f73_3333) || r.flags & 16 != 0
        };
        let first = records
            .surface
            .iter()
            .chain(&reduction.inactive_records)
            .take(records.inserted_surface_count)
            .find(condition);
        if first.is_some_and(|r| dot3(sub(r.position, input.position), input.surface_up) > 0.) {
            packet.flags |= 512;
        }
        let first = records.obstacle.iter().find(condition);
        if first.is_some_and(|r| dot3(sub(r.position, input.position), input.surface_up) > 0.) {
            packet.flags |= 512;
        }
    }
    let mut remaining = length_inverse(input.velocity).0 * 0.25;
    for s in &segments.items {
        if s.kind != Kind::Surface && !(s.normal[1] > f32::from_bits(0x3f66_6666)) {
            packet.edge_position = s.end;
            packet.flags |= 4;
            break;
        }
        packet.edge_normal = s.normal;
        remaining -= s.length;
        if remaining <= 0. {
            packet.edge_position =
                std::array::from_fn(|i| s.direction[i].mul_add(remaining, s.end[i]));
            break;
        }
    }
}

fn publish_slope(packet: &mut Packet, tangent: V, history: &mut History) {
    // The native scalar path fuses x*x + z*z and clamps its horizontal length.
    let sq = tangent[0].mul_add(tangent[0], tangent[2] * tangent[2]);
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(sq);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-sq).mul_add(inverse * inverse, 1.), inverse);
    }
    let length = if sq == 0. { 0. } else { sq * inverse };
    let minimum = f32::from_bits(0x3a83_126f);
    let denominator = if minimum - length >= 0. {
        minimum
    } else {
        length
    };
    let slope = (1. / denominator) * tangent[1];
    packet.value_160 = slope;
    let tan = crate::animation::foot_ik::post_contact::tangent;
    let low = tan(f32::from_bits(0x3dd6_7750));
    let high = tan(f32::from_bits(0x3f0e_fa35));
    let offset = if packet.flags & 64 != 0 { 0 } else { 4 };
    let kind = if !(slope.abs() <= high) {
        offset + if slope > 0. { 2 } else { 4 }
    } else if !(slope.abs() <= low) {
        offset + if slope > 0. { 1 } else { 3 }
    } else {
        0
    };
    if kind == 0 {
        history.cursor = 0;
        history.stable_kind = 0;
        history.samples = [0; 3];
        packet.kind_164 = 0;
        return;
    }
    if history.samples.iter().all(|&previous| previous == kind) {
        history.stable_kind = kind;
        packet.kind_164 = kind;
    } else {
        history.cursor = (history.cursor + 1) % 3;
        history.samples[history.cursor] = kind;
        packet.kind_164 = history.stable_kind;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slope_kind_requires_three_prior_matching_samples_and_flat_resets_history() {
        let mut packet = Packet {
            flags: 64,
            ..Packet::default()
        };
        let mut history = History::default();
        for expected in [0, 0, 0, 2] {
            publish_slope(&mut packet, [0., 1., 1., 0.], &mut history);
            assert_eq!(packet.kind_164, expected);
        }
        publish_slope(&mut packet, [0., 0., 1., 0.], &mut history);
        assert_eq!(packet.kind_164, 0);
        assert_eq!(history.samples, [0; 3]);
        assert_eq!(history.cursor, 0);
    }
}
