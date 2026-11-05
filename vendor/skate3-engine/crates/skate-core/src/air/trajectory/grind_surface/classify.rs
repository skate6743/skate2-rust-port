use super::*;

/// 82C20ED4..82C213F4, including the helpers 82C21410/21558/216A0.
pub(super) fn resolve(
    plan: &Investigation,
    hits: &[Option<ProbeHit>; 7],
    previous_flags: u32,
) -> GrindSurface {
    let [h0, h1, h2, h3, h4, h5, supplied_h6] = *hits;
    let h6 = if plan.probes.len() == 7 {
        supplied_h6
    } else {
        None
    };
    let blocked = (h0.is_some() && h1.is_some()) || h5.is_some();
    let kind = if blocked {
        GeometryType::Impossible
    } else if h0.is_some() {
        if h2.is_some_and(|h| h.fraction < f32::from_bits(0x3f26_6666)) {
            GeometryType::Ledge
        } else {
            GeometryType::FatRail
        }
    } else if h1.is_some() {
        if h3.is_some_and(|h| h.fraction < f32::from_bits(0x3f26_6666)) {
            GeometryType::Ledge
        } else {
            GeometryType::FatRail
        }
    } else {
        GeometryType::ThinRail
    };
    // Native result high side uses unguarded normalization, not NormalizeSafe.
    let side = cross(UP, plan.direction);
    let side = scale(side, inverse_length(dot(side, side)));
    let high_side = if !blocked && h0.is_none() && h1.is_some() {
        scale(side, -1.)
    } else {
        side
    };
    let curb = kind == GeometryType::Ledge && is_curb(h2, h3, plan.upmost_normal, plan.center);
    let stair = [h0, h1]
        .into_iter()
        .flatten()
        .any(|h| h.packed_surface & 0xf80 == 0x400);
    let optional_normal = h6.is_some_and(|h| optional_normal_test(h, plan.center, plan.direction));
    let optional_drop = h6.is_none_or(|h| !(h.position[1] - plan.center[1] > -1.));
    let mut flags = previous_flags
        & !(GrindSurface::INVALID
            | GrindSurface::CURB
            | GrindSurface::STAIR
            | GrindSurface::OPTIONAL_NORMAL_TEST
            | GrindSurface::OPTIONAL_DROP_TEST);
    if blocked {
        flags |= GrindSurface::BLOCKED_CROSS_SECTION;
    }
    if curb {
        flags |= GrindSurface::CURB;
    }
    if stair {
        flags |= GrindSurface::STAIR;
    }
    if optional_normal {
        flags |= GrindSurface::OPTIONAL_NORMAL_TEST;
    }
    if optional_drop {
        flags |= GrindSurface::OPTIONAL_DROP_TEST;
    }
    let (audio_surface, physics_surface) = surface_info(kind, curb, [h0, h1, h2, h3, h4]);
    let far_points = [
        h2.map_or(plan.probes[2].end, |h| h.position),
        h3.map_or(plan.probes[3].end, |h| h.position),
    ];
    let negative_up = scale(plan.upmost_normal, -1.);
    // Native first computes lengths, then separately refines their reciprocals.
    // No safe-normalize epsilon is introduced for the far-point directions.
    let angles = far_points.map(|point| {
        let delta = sub(point, plan.center);
        let unit = scale(delta, reciprocal(length(delta)));
        crate::trigonometry::acos(dot(negative_up, unit).max(-1.).min(1.))
    });
    let half_pi = f32::from_bits(0x3fc9_0fdb);
    let clearance = f32::from_bits(0x3e8e_fa35);
    let normal_limits = [
        (angles[0] + half_pi) + clearance,
        ((f32::from_bits(0x40c9_0fdb) - angles[1]) - half_pi) - clearance,
    ];
    let tilted_upmost_normal =
        orientation::tilted_normal(plan.direction, plan.upmost_normal, normal_limits);
    GrindSurface {
        center: plan.center,
        far_points,
        upmost_normal: plan.upmost_normal,
        direction: plan.direction,
        high_side,
        normal_limits,
        kind,
        audio_surface,
        physics_surface,
        flags,
        tilted_upmost_normal,
    }
}

fn is_curb(a: Option<ProbeHit>, b: Option<ProbeHit>, up: V, center: V) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return false;
    };
    if !(dot(up, a.normal) > 0.99 && dot(up, b.normal) > 0.99) {
        return false;
    }
    let ah = dot(sub(a.position, center), up);
    let bh = dot(sub(b.position, center), up);
    let (high, low) = if ah >= bh { (ah, bh) } else { (bh, ah) };
    high > -0.035 && high < 0.035 && low > -0.24 && low < -0.2
}

fn optional_normal_test(hit: ProbeHit, center: V, direction: V) -> bool {
    // 82C1E170 subtracts ORIGINAL direction times a positive normalized dot.
    let component = dot(hit.normal, normalize(direction));
    let projected = if component > 0. {
        sub(hit.normal, scale(direction, component))
    } else {
        hit.normal
    };
    let normal = normalize(projected);
    if !(f32::from_bits(0x3f66_6666) > normal[1]) {
        return false;
    }
    let mut offset = sub(hit.position, center);
    offset[1] = 0.;
    dot(offset, normal) > 0.
}

fn surface_info(kind: GeometryType, curb: bool, hits: [Option<ProbeHit>; 5]) -> (u32, u32) {
    if curb {
        return (4, 2);
    }
    if kind == GeometryType::ThinRail {
        return (hits[4].map_or(11, |h| h.packed_surface & 0x7f), 4);
    }
    // Pair priority 0/1, then 2/3, then 4. Equal heights select the second.
    for pair in [0, 2] {
        let selected = match (hits[pair], hits[pair + 1]) {
            (Some(a), Some(b)) => Some(if a.position[1] > b.position[1] { a } else { b }),
            (a, b) => a.or(b),
        };
        if let Some(hit) = selected {
            return unpack(hit.packed_surface);
        }
    }
    hits[4].map_or((3, 1), |h| unpack(h.packed_surface))
}

fn unpack(surface: u32) -> (u32, u32) {
    (surface & 0x7f, (surface >> 7) & 0x1f)
}
