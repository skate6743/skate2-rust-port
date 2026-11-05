//! TU3 82D837C8: ordered rising/falling profile candidate production.
use super::candidate::{Builder, Candidate, between_slope, down_slope, up_slope};
use super::profile::Profile;
use super::{ContactPrefix, Input, cross, dot, length, scale, sub};

pub(super) fn generate(
    input: Input,
    profile: &Profile,
    prefix: &ContactPrefix,
    obstruction: f32,
    retained_flags: u32,
) -> Vec<Candidate> {
    let mut builder = Builder {
        input,
        profile,
        obstruction,
        candidates: Vec::new(),
    };
    let supported = prefix.flags_176 & 1 != 0;
    if supported {
        let planar = sub(
            input.velocity,
            scale(input.up, dot(input.up, input.velocity)),
        );
        if length(planar) < 0.01 {
            builder.add(
                prefix.position,
                prefix.normal,
                cross(input.right, prefix.normal),
                -1,
                0,
                0,
            );
            return builder.candidates;
        }
    }
    if profile.last_ground == 0 {
        return builder.candidates;
    }
    let mut highest: Option<usize> = None;
    let mut have_rise = false;
    let mut saw_drop = false;
    let forward = |point| dot(sub(point, input.position), input.forward);
    for i in 0..profile.last_ground {
        let segment = profile.segments[i];
        let normal = if i + 1 < profile.last_ground {
            let next = profile.segments[i + 1];
            if segment.kind != 0 && next.kind != 0 {
                input.up
            } else {
                next.normal
            }
        } else {
            segment.normal
        };
        if !saw_drop {
            let prior = highest.map_or(input.position, |j| profile.segments[j].end);
            if dot(sub(segment.end, prior), input.up) > 0. {
                highest = Some(i);
            }
        }
        if segment.kind == 1 && segment.length > 0.05 {
            let tall = segment.length > 0.4;
            let mut combined = false;
            let mut eligible = false;
            let mut slope = up_slope(input, segment.length);
            let mut j = i + 1;
            while j < profile.last_ground {
                let next = profile.segments[j];
                if !eligible && next.kind == 2 {
                    eligible = forward(next.start) < obstruction - 0.35;
                }
                if next.kind != 0 && (next.kind != 1 || next.length >= 0.05) {
                    break;
                }
                if !eligible {
                    eligible = dot(next.direction, input.forward) > 0.70700002
                        && forward(next.start) < obstruction - 0.35;
                }
                j += 1;
            }
            if j < profile.last_ground && profile.segments[j].kind == 1 {
                let next_end = profile.segments[j].end;
                let joined = between_slope(input, segment.end, next_end);
                if joined > up_slope(input, segment.length) * 0.5 {
                    slope = joined;
                    combined = true;
                    if !have_rise {
                        combined = between_slope(input, input.position, next_end)
                            >= up_slope(input, segment.length) * 0.5;
                    }
                }
            }
            if !have_rise {
                have_rise = eligible;
            }
            if eligible {
                builder.approach_up(i, if tall { 0x80 } else { 0 }, slope);
                let flags = 0x40 | if combined || !tall { 0 } else { 0x100 };
                builder.point(
                    segment.end,
                    normal,
                    i as i32,
                    flags,
                    if combined { 2 } else { 3 },
                );
            }
        } else if segment.kind == 2 && segment.length > 0.05 {
            let tall = segment.length > 0.4;
            let mut combined = false;
            let mut slope = -down_slope(input, segment.length);
            let mut j = i + 1;
            while j < profile.last_ground {
                let next = profile.segments[j];
                if next.kind != 0 && !(next.kind == 2 && next.length < 0.05) {
                    break;
                }
                j += 1;
            }
            if j < profile.last_ground && profile.segments[j].kind == 2 {
                let joined = between_slope(input, segment.start, profile.segments[j].start);
                if joined < down_slope(input, segment.length) * -0.5 {
                    combined = true;
                    slope = joined;
                }
            }
            let mut suppress = false;
            if !saw_drop && !have_rise {
                if let Some(highest) = highest {
                    suppress =
                        dot(sub(profile.segments[highest].end, input.position), input.up) > 0.02;
                    if i == 1 && profile.segments[0].kind == 0 {
                        suppress = false;
                    }
                }
            }
            saw_drop = true;
            if !suppress {
                if combined {
                    builder.point(segment.start, normal, i as i32, 0x40, 5);
                } else if forward(segment.start) <= 0.75 {
                    builder.approach_down(i as i32, if tall { 0x100 } else { 0 }, slope, 4);
                    builder.point(
                        segment.start,
                        normal,
                        i as i32,
                        0x40 | if tall { 0x80 } else { 0 },
                        6,
                    );
                }
            }
        }
    }
    let gap = input.position[1] - prefix.position[1];
    if supported && gap > 0.01 {
        let flags = if gap >= 0.4 {
            retained_flags & 0x100 | 0x40
        } else {
            retained_flags & 0x140
        };
        if builder.candidates.is_empty() || !saw_drop {
            let kind = if builder.candidates.is_empty() { 7 } else { 8 };
            // 82D83F94 loads stock 8208824C=0x3F4CCCCD, not `gap`.
            builder.approach_down(-1, flags, -down_slope(input, 0.8), kind);
        }
    } else if builder.candidates.is_empty() {
        builder.advance(length(input.velocity) * 0.016666668);
    }
    builder.candidates
}
