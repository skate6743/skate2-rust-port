//! PositionSelector82D63D40, velocity window82D64420 and edge clipping82D651E0.
use super::{GRAVITY, Settings, UP, V, math::*};
use skate3_core::animation::foot_ik::transforms::inverse_rigid;
use skate3_core::{
    air::trajectory::Trajectory,
    physics::{grind_contact::Primitive, skeleton_animation_record::IDENTITY},
};

#[derive(Clone, Copy)]
pub(super) struct Candidate {
    pub point: V,
    pub edge: Primitive,
    pub side: i32,
}

pub(super) fn select(
    settings: &Settings,
    com: V,
    velocity: V,
    normal: V,
    hint: i32,
    edges: &[Primitive],
) -> Option<Candidate> {
    //82D61D88: ascending, above minimum speed, slope over45deg and |Ny|<=.95.
    if !(length(velocity) > settings.minimum_speed
        && velocity[1] > 0.0
        && normal[1].abs() <= 0.95
        && skate3_core::trigonometry::acos(normal[1].clamp(-1.0, 1.0)) * f32::from_bits(0x42652ee1)
            > settings.minimum_slope)
    {
        return None;
    }
    let mut frame = IDENTITY;
    if normal[1].abs() <= 0.99 {
        frame[0] = normalize(cross(UP, normal));
        frame[1] = UP;
        frame[2] = cross(frame[0], UP);
    }
    frame[3] = com;
    let inverse = inverse_rigid(&frame);
    let mut velocity = rotate(&inverse, velocity);
    velocity = [
        velocity[0],
        (velocity[0] * velocity[0] + velocity[1] * velocity[1]).sqrt(),
        0.0,
        0.0,
    ];
    if hint != 0 && hint != if velocity[0] > 0.0 { 1 } else { 2 } {
        velocity[0] = if hint == 1 { 1.0 } else { -1.0 } * f32::from_bits(0x37800000);
    }
    let g = settings.window.each_ref().map(|g| g.evaluate(velocity[1]));
    let lo_x = g[1].max(velocity[0].abs() - g[5]);
    let hi_x = g[6].min(velocity[0].abs() + g[4]);
    if hi_x <= lo_x {
        return None;
    }
    let lo_y = g[0].max(velocity[1] - g[3]);
    let hi_y = velocity[1] + g[2];
    let sign = if velocity[0] > 0.0 { 1.0 } else { -1.0 };
    let apex = |x, y| {
        let trajectory = Trajectory {
            position: point(&inverse, com),
            velocity: [x * sign, y, 0.0, 0.0],
            acceleration: GRAVITY,
            duration: -1.0,
        };
        let mut p = trajectory.position_at(super::apex_time(trajectory));
        p[1] -= settings.window_drop;
        p
    };
    //Native order:r5 lowX/lowY,r6 highX/lowY,r7 lowX/highY,r8 highX/highY.
    let [a, b, c, d] = [
        apex(lo_x, lo_y),
        apex(hi_x, lo_y),
        apex(lo_x, hi_y),
        apex(hi_x, hi_y),
    ];
    let polygon = if d[0] > c[0] {
        [c, d, b, a]
    } else {
        [d, c, a, b]
    };
    //82C1EAD8 preserves world order and caps the broadphase result at40.
    let mut candidates = Vec::new();
    for edge in edges
        .iter()
        .filter(|e| {
            (0..3).all(|i| {
                let extent = [2.0, 4.0, 2.0][i];
                e.start[i].min(e.end[i]) <= com[i] + extent
                    && e.start[i].max(e.end[i]) >= com[i] - extent
            })
        })
        .take(40)
    {
        let start = point(&inverse, edge.start);
        let end = point(&inverse, edge.end);
        if normalize(sub(end, start))[0].abs() <= 0.71 {
            continue;
        } //82D64218
        let mut segment = [start, end];
        //82D657D0/658A0 retains -depth<localZ<0.
        if !clip_depth(&mut segment, -settings.depth, true) || !clip_depth(&mut segment, 0.0, false)
        {
            continue;
        }
        let mut valid = true;
        for i in 0..4 {
            let delta = sub(polygon[(i + 1) % 4], polygon[i]);
            let n = normalize([delta[1], -delta[0], 0.0, 0.0]);
            let d0 = dot(sub(segment[0], polygon[i]), n);
            let d1 = dot(sub(segment[1], polygon[i]), n);
            if !clip(&mut segment, d0, d1) {
                valid = false;
                break;
            }
        }
        if valid {
            candidates.push((*edge, segment));
        }
    }
    let mut best = None;
    let mut height = -f32::MAX;
    let mut depth = -f32::MAX;
    for (i, (_, segment)) in candidates.iter().enumerate() {
        let p = if segment[0][1] > segment[1][1] {
            segment[0]
        } else {
            segment[1]
        };
        if p[1] > height + 0.1 || (p[1] > height - 0.1 && p[1] <= height + 0.1 && p[2] > depth) {
            best = Some(i);
            height = p[1];
            depth = p[2];
        }
    }
    let best = best?;
    let mut connected = vec![false; candidates.len()];
    connected[best] = true;
    //82D65BD8: extend both ends across neighbours within .05m and30degrees.
    for reverse in [false, true] {
        let mut ends = candidates[best].1;
        if reverse {
            ends.swap(0, 1);
        }
        loop {
            let next = candidates.iter().enumerate().find_map(|(i, (_, s))| {
                if connected[i] {
                    return None;
                }
                for e in 0..2 {
                    if dot(sub(ends[1], s[e]), sub(ends[1], s[e])) < f32::from_bits(0x3b23d70a)
                        && skate3_core::trigonometry::acos(
                            dot(
                                normalize(sub(ends[1], ends[0])),
                                normalize(sub(s[1 - e], s[e])),
                            )
                            .clamp(-1.0, 1.0),
                        )
                        .abs()
                            < f32::from_bits(0x3f060a92)
                    {
                        return Some((i, [s[e], s[1 - e]]));
                    }
                }
                None
            });
            let Some((i, s)) = next else { break };
            connected[i] = true;
            ends = s;
        }
    }
    let bottom = scale(add(polygon[2], polygon[3]), 0.5);
    let top = scale(add(polygon[0], polygon[1]), 0.5);
    let delta = sub(top, bottom);
    let target = madd(
        delta,
        clamp01((height - bottom[1]) * reciprocal(delta[1])),
        bottom,
    );
    let target = [target[0], target[1], 0.0, 0.0];
    let mut nearest = [0.0; 4];
    let mut distance = f32::MAX;
    for (i, (_, segment)) in candidates.iter().enumerate() {
        if !connected[i] {
            continue;
        }
        let delta = sub(segment[1], segment[0]);
        let square = dot(delta, delta);
        let p = if square > f32::from_bits(0x37800000) {
            madd(
                delta,
                clamp01(dot(sub(target, segment[0]), delta) * reciprocal(square)),
                segment[0],
            )
        } else {
            segment[0]
        };
        let square = dot(sub(p, target), sub(p, target));
        if square < distance {
            distance = square;
            nearest = p;
        }
    }
    Some(Candidate {
        point: point(&frame, nearest),
        edge: candidates[best].0,
        side: if velocity[0] > 0.0 { 1 } else { 2 },
    })
}
//82D658A0 excludes coplanar segments and avoids dividing near-parallel Z.
fn clip_depth(segment: &mut [V; 2], value: f32, above: bool) -> bool {
    let sign = if above { 1.0 } else { -1.0 };
    let a = (segment[0][2] - value) * sign;
    let b = (segment[1][2] - value) * sign;
    if a <= 0.0 && b <= 0.0 {
        return false;
    }
    let delta = segment[1][2] - segment[0][2];
    if delta.abs() >= f32::from_bits(0x37800000) {
        let t = (value - segment[0][2]) * reciprocal(delta);
        if t > 0.0 && t < 1.0 {
            let p = madd(sub(segment[1], segment[0]), t, segment[0]);
            segment[usize::from(b <= 0.0)] = p;
        }
    }
    true
}
fn clip(segment: &mut [V; 2], a: f32, b: f32) -> bool {
    if a < 0.0 && b < 0.0 {
        return false;
    }
    if (a < 0.0) != (b < 0.0) {
        let p = madd(
            sub(segment[1], segment[0]),
            a * reciprocal(a - b),
            segment[0],
        );
        segment[usize::from(a >= 0.0)] = p;
    }
    true
}
