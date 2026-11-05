//! Contact-list infill82D82498, normal correction82D82940/82D82AF0.
use super::analyzer_math::{clamp_normal, madd, plane_segment};
use super::profile::sort;
use super::{ContactSample, Input, Samples, dot, length, scale, sub};

pub(super) fn insert_obstacles(input: Input, s: &mut Samples) {
    let initial = s.ground.clone();
    for pair in initial.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let mut low = a.height.min(b.height);
        let mut high = a.height.max(b.height);
        let mut i = 0;
        while i < s.other.len() {
            let p = s.other[i];
            if a.forward_distance > p.forward_distance || p.forward_distance > b.forward_distance {
                i += 1;
                continue;
            }
            if low > p.height {
                s.other[i].flags |= 2;
                i += 1;
                continue;
            }
            if high > p.height
                || p.sort_distance < a.sort_distance
                || p.sort_distance > b.sort_distance
            {
                i += 1;
                continue;
            }
            let mut minimum = i;
            let mut maximum = i;
            s.other[i].flags |= 2;
            i += 1;
            while i < s.other.len()
                && (p.forward_distance - s.other[i].forward_distance).abs() <= 0.001
            {
                let q = s.other[i];
                s.other[i].flags |= 2;
                if q.height >= high && s.other[minimum].height > q.height {
                    minimum = i;
                }
                if q.height > s.other[maximum].height {
                    maximum = i;
                }
                i += 1;
            }
            for index in [minimum, maximum] {
                s.other[index].flags = (s.other[index].flags & !2) | 1;
            }
            let p = s.other[minimum];
            s.insert(input, p.position, p.normal, 1, p.sort_distance, 1);
            if p.height > high {
                low = high;
                high = p.height;
            } else if p.height > low {
                low = p.height;
            }
            if minimum != maximum {
                let q = s.other[maximum];
                s.insert(input, q.position, q.normal, 1, q.sort_distance, 1);
            }
            // 82D82498 advances once more at the outer for-loop increment,
            // after the equal-forward-distance group has already advanced i.
            i += 1;
        }
    }
    sort(&mut s.ground);
}
pub(super) fn correct_normals(input: Input, s: &mut Samples) {
    let mut previous = input.position;
    for p in &mut s.ground {
        let height = dot(sub(p.position, previous), input.up);
        p.normal = if height.abs() < 0.001 {
            input.up
        } else {
            clamp_normal(
                if height <= 0. {
                    input.forward
                } else {
                    scale(input.forward, -1.)
                },
                input.up,
                p.normal,
            )
        };
        previous = p.position;
    }
    let initial = s.ground.clone();
    for pair in initial.windows(2) {
        for i in 0..s.other.len() {
            let p = s.other[i];
            if p.flags & 3 == 0
                && pair[0].forward_distance <= p.forward_distance
                && p.forward_distance <= pair[1].forward_distance
            {
                infill(input, s, pair[0], pair[1], i);
            }
        }
    }
    sort(&mut s.ground);
}
fn infill(input: Input, s: &mut Samples, a: ContactSample, b: ContactSample, index: usize) {
    let p = s.other[index];
    if dot(a.normal, b.normal) > 0.99
        && dot(a.normal, p.normal) > 0.99
        && dot(sub(b.position, a.position), b.normal).abs() < 0.01
    {
        let mut delta = sub(p.position, a.position);
        if length(delta) < 0.01 {
            delta = sub(p.position, b.position);
        }
        if dot(delta, b.normal).abs() < 0.01 {
            return;
        }
    }
    let n = clamp_normal(
        if b.height > a.height {
            scale(input.forward, -1.)
        } else {
            input.forward
        },
        input.up,
        p.normal,
    );
    let delta = sub(b.position, a.position);
    let first_end = madd(
        sub(delta, scale(a.normal, dot(a.normal, delta))),
        1.1,
        a.position,
    );
    let last_start = sub(
        b.position,
        scale(sub(delta, scale(b.normal, dot(b.normal, delta))), 1.1),
    );
    for candidate in [
        plane_segment(p.position, n, a.position, first_end),
        plane_segment(p.position, n, last_start, b.position),
    ]
    .into_iter()
    .flatten()
    {
        let d = sub(candidate, input.position);
        let x = dot(input.forward, d);
        let y = dot(input.up, d);
        if x >= a.forward_distance
            && x <= b.forward_distance
            && y >= a.height.min(b.height)
            && y <= a.height.max(b.height)
        {
            let t = length(sub(candidate, a.position)) / length(delta);
            let order = t * (b.sort_distance - a.sort_distance) + a.sort_distance;
            s.insert(input, candidate, n, 1, order, 2);
            s.other[index].flags |= 4;
        }
    }
}
