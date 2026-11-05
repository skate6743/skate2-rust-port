use super::*;
pub(super) struct Contacts {
    pub times: [f32; 5],
    pub distances: [f32; 5],
    pub corrections: [V; 5],
    earliest: f32,
}
pub(super) fn contacts(
    samples: &prediction::Samples,
    count: usize,
    origin: V,
    rail: V,
    normal: V,
) -> Contacts {
    let mut out = Contacts {
        times: [-1.; 5],
        distances: [0.; 5],
        corrections: [ZERO; 5],
        earliest: count as f32,
    };
    for p in 0..5 {
        for i in 1..count {
            let a = dot(normal, sub(samples[i - 1][p], origin));
            let b = dot(normal, sub(samples[i][p], origin));
            if a * b < 0. {
                let f = (a / (a - b)).abs();
                let at = madd(samples[i - 1][p], 1. - f, scale(samples[i][p], f));
                let on = madd(rail, dot(sub(at, origin), rail), origin);
                out.times[p] = (i - 1) as f32 + f;
                out.corrections[p] = sub(on, at);
                out.distances[p] = length(out.corrections[p]);
                out.earliest = out.earliest.min(out.times[p]);
                break;
            }
        }
    }
    out
}
pub(super) fn choose(
    c: &Contacts,
    headings: &[f32; 12],
    s: &Settings,
    inverted: bool,
    previous: Option<usize>,
) -> Option<usize> {
    let valid = |kind: usize| {
        let p = targets::CONTACTS[kind];
        let t = c.times[p];
        if kind == 4 {
            if c.times[2].min(c.times[3]) < 4. && (c.times[2] - c.times[3]).abs() > 2. {
                return false;
            }
        } else if c.earliest < 5. && (c.earliest - t).abs() > 3. {
            return false;
        }
        t > 0.
            && c.distances[p] < s.distances[kind]
            && (targets::DESIRED[kind] - headings[(t + 0.5) as usize].abs()).abs() < s.ranges[kind]
    };
    let mut selected: Option<usize> = None;
    for mut kind in 0..7 {
        if inverted != (kind == 0) || !valid(kind) {
            continue;
        }
        if kind == 3 {
            if let Some(old @ (1 | 2)) = previous {
                if valid(old) {
                    kind = old;
                }
            }
        }
        if selected.is_none_or(|old| {
            c.distances[targets::CONTACTS[kind]] < c.distances[targets::CONTACTS[old]]
        }) {
            selected = Some(kind);
        }
        if selected.is_some_and(|kind| kind < 5) {
            break;
        }
    }
    selected
}
