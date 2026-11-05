//! Additional TU3 deck/truck contact queries and family admission leaves.
use super::admission::{Admission, EntryKind};
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub geometry: FiftyFiftyCandidate,
    pub front: bool,
    pub kind: u32,
    pub entry_kind: EntryKind,
}

fn rectangle(a: V, b: V, c: V, d: V, reference: V, edges: &[Primitive]) -> Option<TruckContact> {
    let mut best = None;
    let mut distance = 1_000_000.0;
    for (primitive, edge) in edges.iter().enumerate() {
        if let Some(position) = segment_triangle(edge.start, edge.end, [a, b, c])
            .or_else(|| segment_triangle(edge.start, edge.end, [d, a, c]))
        {
            let delta = sub(position, reference);
            let square = dot3(delta, delta);
            if square < distance {
                distance = square;
                best = Some(TruckContact {
                    position,
                    primitive,
                });
            }
        }
    }
    best
}

///82C1F968: inverted deck only, .3 longitudinal half-length, inverted depth.
pub fn inverted_contact(
    board: [V; 4],
    flags: u32,
    epsilon: f32,
    depth: f32,
    edges: &[Primitive],
) -> Option<TruckContact> {
    if flags & 0x0020_0000 == 0 {
        return None;
    }
    //82C1F968 uses the same diagonal as82C1FB98.
    let mut inverted = board;
    inverted[1] = scale(board[1], -1.0);
    deck_contact(inverted, 0, epsilon, depth, 0.3, edges)
}

///82C20130: longitudinal trapezoids beyond each truck, with the native
///current-tip suppression so the loaded end does not swap on each tick.
pub fn tip_contacts(
    board: [V; 4],
    flags_2484: u32,
    state: u32,
    flags_2468: u32,
    truck_distance: f32,
    edges: &[Primitive],
) -> [Option<TruckContact>; 2] {
    if flags_2484 & 0x0020_0000 != 0 {
        return [None; 2];
    }
    core::array::from_fn(|i| {
        if matches!(state, 402 | 404) && ((flags_2468 & 0x200 != 0) != (i == 0)) {
            return None;
        }
        let sign = if i == 0 { 1.0 } else { -1.0 };
        let centre = add(board[3], scale(board[1], 0.02));
        let a = add(centre, scale(board[2], sign * truck_distance));
        let b = add(
            add(centre, scale(board[2], sign * (truck_distance + 0.215))),
            scale(board[1], 0.03),
        );
        let c = add(b, scale(board[1], -0.2));
        let d = add(a, scale(board[1], -0.2));
        rectangle(a, b, c, d, scale(add(a, b), 0.5), edges)
    })
}

fn geometry(hit: TruckContact, edge: Primitive) -> FiftyFiftyCandidate {
    let delta = sub(edge.end, edge.start);
    FiftyFiftyCandidate {
        direction: scale(delta, arithmetic::inverse_square_root(dot3(delta, delta))),
        centre: hit.position,
        front: edge.end,
        rear: edge.start,
        primitive: hit.primitive,
    }
}

///82D883E0: a tip on a height-discontinuous edge becomes a backslash when
///the deck lies over the higher side. Existing tipslides remain tipslides.
pub fn is_backslash(
    board_position: V,
    point: V,
    far_points: [V; 2],
    support: V,
    truck_distance: f32,
    state: u32,
) -> bool {
    if state == 402 {
        return false;
    }
    let offsets = far_points.map(|p| sub(p, point));
    let heights = offsets.map(|p| dot3(p, support));
    if (heights[0] - heights[1]).abs() <= truck_distance * 0.1 {
        return false;
    }
    let higher = if heights[0] > heights[1] {
        offsets[0]
    } else {
        offsets[1]
    };
    dot3(sub(board_position, point), higher) > 0.0
}

///82D88E88. Unlike boardslides, inverted decks have no truck-clearance test.
pub fn darkslide(
    board: [V; 4],
    hit: TruckContact,
    edge: Primitive,
    velocity: V,
    category: u32,
    low_wheel_frames: u32,
    admission: &Admission<'_>,
) -> Option<Contact> {
    let mut g = geometry(hit, edge);
    let delta = sub(edge.end, edge.start);
    g.direction = scale(
        delta,
        arithmetic::reciprocal(arithmetic::square_root(dot3(delta, delta))),
    );
    if dot3(upright_normal(g.direction), scale(board[1], -1.0)) <= 0.45 {
        return None;
    }
    //82D8907C deliberately tests state400, even for darkslide kind5.
    let decision = admission.test(400, g.direction, false);
    if !decision.allowed {
        return None;
    }
    let depth = dot3(sub(board[3], hit.position), board[1]);
    if dot3(scale(board[1], depth), scale(board[1], depth)) >= 0.005625 {
        return None;
    }
    if category == 100 && dot3(velocity, g.direction).abs() <= 0.75 && low_wheel_frames <= 10 {
        return None;
    }
    Some(Contact {
        geometry: g,
        front: false,
        kind: 5,
        entry_kind: decision.kind,
    })
}

///82D89898/82D89820: valid single-truck contacts. Selection between two valid
///ends is performed below using the native travel-relative branches.
pub fn five_o(
    board: [V; 4],
    hits: [Option<TruckContact>; 2],
    edges: &[Primitive],
    velocity: V,
    flags_2476: u32,
    flags_2472: u32,
    ground_frames: u32,
    translation: f32,
    admission: &Admission<'_>,
) -> Option<Contact> {
    if flags_2476 & 0x4000_0000 != 0 {
        return None;
    }
    let valid = hits.map(|hit| {
        hit.and_then(|hit| {
            let g = geometry(hit, edges[hit.primitive]);
            let depth = dot3(sub(board[3], hit.position), board[1]);
            let projected = scale(board[1], depth);
            if dot3(projected, projected) >= f32::from_bits(0x3c8a71de) {
                return None;
            }
            let decision = admission.test(403, g.direction, true);
            if !decision.allowed
                || dot3(board[0], g.direction).abs() >= 0.906
                || dot3(board[1], g.direction).abs() >= 0.46
            {
                return None;
            }
            Some((g, decision.kind))
        })
    });
    let forwards = dot3(board[2], velocity) > 0.0;
    let leading = if forwards { 0 } else { 1 };
    let trailing = 1 - leading;
    let (take_leading, take_trailing) = match (valid[leading].is_some(), valid[trailing].is_some())
    {
        (true, true) => {
            let lead = if ground_frames > 30 {
                flags_2472 & 0x1000 != 0
            } else {
                translation > 0.68
            };
            (lead, !lead)
        }
        (false, true) => (false, true),
        (true, false) => (ground_frames <= 30 || flags_2472 & 0x1000 != 0, false),
        _ => (false, false),
    };
    let index = if take_trailing {
        trailing
    } else if take_leading {
        leading
    } else {
        return None;
    };
    Some(Contact {
        geometry: valid[index]?.0,
        front: index == 0,
        kind: 3,
        entry_kind: valid[index]?.1,
    })
}

///82D89428 selects the positive tip first, then checks penetration, descent,
///cross-rail velocity and the animation's lateral balance intent.
pub fn tipslide(
    board: [V; 4],
    hits: [Option<TruckContact>; 2],
    edges: &[Primitive],
    velocity: V,
    category: u32,
    flags_2476: u32,
    flags_2472: u32,
    ground_frames: u32,
    balance: f32,
    reference_right: V,
    admission: &Admission<'_>,
) -> Option<Contact> {
    if flags_2476 & 0x4000_0000 != 0 {
        return None;
    }
    let front = hits[0].is_some();
    let hit = if front { hits[0]? } else { hits[1]? };
    let g = geometry(hit, edges[hit.primitive]);
    let decision = admission.test(402, g.direction, true);
    if !decision.allowed {
        return None;
    }
    if (decision.kind == EntryKind::DropIn || velocity[1] < 0.0 && category != 400)
        && board[3][1] <= hit.position[1]
    {
        return None;
    }
    let delta = sub(board[3], hit.position);
    let depth = dot3(delta, board[1]);
    let projected = scale(board[1], depth);
    if dot3(projected, projected) >= f32::from_bits(0x3b6bedfa) {
        return None;
    }
    let perpendicular = sub(velocity, scale(g.direction, dot3(g.direction, velocity)));
    if dot3(perpendicular, perpendicular) < 9.0 {
        if balance.abs() > 0.0 {
            if dot3(sub(hit.position, board[3]), reference_right) >= 0.0 {
                return None;
            }
        } else if ground_frames > 30 {
            let toward = dot3(sub(hit.position, board[3]), velocity) > 0.0;
            if (flags_2472 & 0x1000 != 0) != toward {
                return None;
            }
        }
    }
    Some(Contact {
        geometry: g,
        front,
        kind: 2,
        entry_kind: decision.kind,
    })
}
