use super::ContactRecord;
use super::arithmetic::{V, cross, dot, scale, sub, vector};

/// Complete geometric predicate 8277A208. Body IDs are checked by its caller.
/// The native comparison uses the *smaller* absolute plane displacement.
pub fn coplanar_contacts(a: &ContactRecord, b: &ContactRecord) -> bool {
    let normal = vector(a, 8);
    if f32::from_bits(0x3f7f_be77) > dot(normal, vector(b, 8)) {
        return false;
    }
    let da = dot(normal, sub(vector(a, 0), vector(b, 0))).abs();
    let db = dot(normal, sub(vector(a, 4), vector(b, 4))).abs();
    let selected = if da - db >= 0.0 { db } else { da };
    selected < f32::from_bits(0x3c23_d70a)
}

/// Full 827798F8 point selection. Inputs are paired world-space positions;
/// output slots beyond the returned count retain the native write behavior.
/// The caller provides at least one pair, as the native function requires.
pub fn select_contact_points(a: &[V], b: &[V], normal: V, output: &mut [u32; 4]) -> usize {
    assert!(!a.is_empty() && a.len() == b.len());
    let mut first = 0;
    let mut origin = a[0];
    let mut depth = dot(normal, sub(a[0], b[0]));
    for i in 1..a.len() {
        let candidate = dot(normal, sub(a[i], b[i]));
        if depth > candidate {
            depth = candidate;
            first = i;
            origin = a[i];
        }
    }
    let mut third_delta = sub(a[0], origin);
    let mut far_delta = third_delta;
    let mut far = 0;
    let mut distance = dot(far_delta, far_delta);
    for (i, &point) in a.iter().enumerate().skip(1) {
        let delta = sub(point, origin);
        let candidate = dot(delta, delta);
        if candidate > distance {
            distance = candidate;
            far = i;
            far_delta = delta;
        }
    }
    if first == far {
        output[0] = first as u32;
        return 1;
    }
    let mut third = 0;
    let initial_area = cross(third_delta, far_delta);
    let mut area = dot(initial_area, initial_area);
    for (i, &point) in a.iter().enumerate().skip(1) {
        let delta = sub(point, origin);
        let area_vector = cross(delta, far_delta);
        let candidate = dot(area_vector, area_vector);
        if candidate > area {
            area = candidate;
            third = i;
            third_delta = delta;
        }
    }
    output[..3].copy_from_slice(&[first as u32, far as u32, third as u32]);
    if third == first || third == far {
        return 2;
    }
    let side = cross(normal, far_delta);
    let sign = if dot(side, third_delta) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let far_side = scale(side, sign);
    let third_side = scale(cross(normal, third_delta), sign);
    let edge = sub(third_delta, far_delta);
    let mut fourth = 9999;
    let mut best = -1.0;
    for (i, &point) in a.iter().enumerate() {
        let delta = sub(point, origin);
        let selected = if dot(delta, far_side) > 0.0 {
            if dot(delta, third_side) > 0.0 {
                cross(delta, third_delta)
            } else {
                cross(sub(delta, far_delta), edge)
            }
        } else {
            cross(delta, far_delta)
        };
        let candidate = dot(selected, selected);
        if candidate > best {
            best = candidate;
            fourth = i;
        }
    }
    output[3] = fourth as u32;
    3 + usize::from(fourth != first && fourth != far && fourth != third && fourth != 9999)
}

/// Complete 82779EB0 grouping/reduction. Retained contacts are restored to
/// original order; discarded tail records are not cleared. No drop count change.
pub(super) fn reduce(records: &mut [ContactRecord; 50], count: &mut u32) {
    let original = *count as usize;
    assert!(original <= 50);
    let mut assigned = [false; 50];
    let mut retained = Vec::with_capacity(original);
    for first in 0..original {
        if assigned[first] {
            continue;
        }
        assigned[first] = true;
        let mut group = vec![first];
        for next in first + 1..original {
            if !assigned[next]
                && records[first][3] == records[next][3]
                && records[first][7] == records[next][7]
                && coplanar_contacts(&records[first], &records[next])
            {
                assigned[next] = true;
                group.push(next);
            }
        }
        if group.len() <= 4 {
            retained.extend(group);
        } else {
            let a: Vec<_> = group.iter().map(|&i| vector(&records[i], 0)).collect();
            let b: Vec<_> = group.iter().map(|&i| vector(&records[i], 4)).collect();
            let mut selected = [0, 1, 2, 3];
            let length = select_contact_points(&a, &b, vector(&records[first], 8), &mut selected);
            retained.extend(selected[..length].iter().map(|&i| group[i as usize]));
        }
    }
    if retained.len() < original {
        retained.sort_unstable();
        *count = retained.len() as u32;
        for (destination, source) in retained.into_iter().enumerate() {
            if destination != source {
                // Native 8277A300 loads/stores float fields individually. Its
                // f32→f64→f32 copy quiets sNaNs; IDs/tag remain integer stores.
                records[destination] = records[source].map(|bits| {
                    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
                        bits | 0x0040_0000
                    } else {
                        bits
                    }
                });
                for i in [3, 7, 23] {
                    records[destination][i] = records[source][i];
                }
            }
        }
    }
}
