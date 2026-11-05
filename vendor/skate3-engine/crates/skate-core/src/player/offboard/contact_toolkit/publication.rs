//! TU3 82D84060: profile lookahead, candidate selection and prefix publication.
use super::analyzer_math::madd;
use super::candidate::Candidate;
use super::classification::History;
use super::profile::Profile;
use super::{ContactPrefix, Input, Samples, dot, length, sub};

pub(super) fn publish(
    input: Input,
    profile: &Profile,
    samples: &Samples,
    candidates: &mut [Candidate],
    obstruction: f32,
    retained: &mut Candidate,
    history: &mut History,
    prefix: &mut ContactPrefix,
) {
    prefix.edge_position = input.position;
    prefix.edge_normal = input.up;
    prefix.distance_168 = 0.;
    if profile.last_ground != 0 {
        let end = profile.segments[profile.last_ground - 1].end;
        let reach = dot(sub(end, input.position), input.forward);
        prefix.distance_168 = if reach >= 1.65 { 1e10 } else { reach };
        if let Some(segment) = profile
            .segments
            .iter()
            .find(|s| (s.kind == 1 || s.kind == 2) && s.length > 0.05)
        {
            prefix.distance_168 = dot(sub(segment.start, input.position), input.forward);
        }
        if prefix.flags_176 & 1 != 0 && input.forward[1] > 0.2 && prefix.normal[1] < 0.7 {
            for points in [
                &samples.ground[..samples.original_ground_count.min(samples.ground.len())],
                &samples.other[..],
            ] {
                if let Some(point) = points
                    .iter()
                    .find(|p| p.normal[1] > 0.95 || p.flags & 0x10 != 0)
                {
                    if dot(sub(point.position, input.position), input.up) > 0. {
                        prefix.flags_176 |= 0x200;
                    }
                }
            }
        }
        let mut distance = length(input.velocity) * 0.25;
        for segment in &profile.segments {
            if segment.kind != 0 && segment.normal[1] <= 0.9 {
                continue;
            }
            prefix.edge_normal = segment.normal;
            distance -= segment.length;
            if distance <= 0. {
                prefix.edge_position = madd(segment.direction, distance, segment.end);
                break;
            }
            prefix.edge_position = segment.end;
            prefix.flags_176 |= 4;
        }
    }
    candidates.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if candidates.is_empty() {
        if obstruction >= 0.3 {
            prefix.flags_176 &= !1;
        } else {
            prefix.flags_176 |= 0x400;
        }
        return;
    }
    let step = length(input.velocity) * 0.016666668;
    let mut selected = candidates.len() - 1;
    for (index, candidate) in candidates.iter().enumerate() {
        let behind = index + 1 < candidates.len()
            && dot(input.forward, sub(candidate.position, input.position)) < step;
        if candidate.kind == 1 {
            let overridden = behind
                || candidates[index + 1..]
                    .iter()
                    .take_while(|next| next.low <= candidate.high)
                    .any(|next| matches!(next.kind, 4 | 7 | 8));
            if overridden {
                selected = candidates[index + 1..]
                    .iter()
                    .position(|next| next.segment == candidate.segment)
                    .map_or(index, |offset| index + 1 + offset);
                break;
            }
        }
        if !behind {
            selected = index;
            break;
        }
    }
    *retained = candidates[selected];
    prefix.target_position = retained.position;
    prefix.target_normal = retained.normal;
    prefix.flags_176 |= retained.flags | 2;
    if prefix.flags_176 & 1 == 0 {
        prefix.flags_176 |= 0x3b;
        prefix.position = input.position;
        prefix.normal = input.up;
    }
    history.classify(prefix, retained.direction);
}
