//! Static geometry investigation from original S3 TU3 82C20728/82C20C08.
//! Query execution is supplied by the world owner; a miss is not missing data.
//! No moving-object observations or authored material codes are synthesized.
use super::math::*;

mod classify;
mod landing;
mod orientation;
pub use landing::{LandingOrientation, update_landing_orientation};
#[cfg(test)]
mod tests;

pub type V = [f32; 4];

#[derive(Clone, Copy, Debug)]
pub struct InvestigationInput {
    pub start: V,
    pub end: V,
    pub reference: V,
    /// Input +48, enabled by input byte +64: a six-unit world-down probe.
    pub optional_probe: Option<V>,
    /// physics_grinds layout +636, supplied from the stock collection.
    pub deck_center_to_truck: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Probe {
    pub start: V,
    pub end: V,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ProbeHit {
    pub fraction: f32,
    pub position: V,
    pub normal: V,
    /// Original query result +100, NOT a rendering tag or triangle index.
    pub packed_surface: u32,
}

#[derive(Clone, Debug)]
pub struct Investigation {
    pub center: V,
    pub upmost_normal: V,
    pub direction: V,
    pub probes: Vec<Probe>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GeometryType {
    ThinRail = 0,
    FatRail = 1,
    Ledge = 2,
    Impossible = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct GrindSurface {
    pub center: V,
    pub far_points: [V; 2],
    pub upmost_normal: V,
    pub direction: V,
    pub high_side: V,
    pub normal_limits: [f32; 2],
    pub kind: GeometryType,
    pub audio_surface: u32,
    pub physics_surface: u32,
    /// Native output +116. Preserve unnamed bits, rather than invent meaning.
    pub flags: u32,
    /// 82C21828 evaluated from the geometric limits; distinct from +48 upmost.
    pub tilted_upmost_normal: V,
}

impl GrindSurface {
    pub const INVALID: u32 = 0x8000_0000;
    pub const CURB: u32 = 0x4000_0000;
    pub const BLOCKED_CROSS_SECTION: u32 = 0x2000_0000;
    pub const STAIR: u32 = 0x1000_0000;
    pub const OPTIONAL_NORMAL_TEST: u32 = 0x0800_0000;
    pub const OPTIONAL_DROP_TEST: u32 = 0x0400_0000;

    /// 82C20C08's no-submission output; not an artificial startup blocker.
    fn not_submitted(input: InvestigationInput) -> Self {
        let distance = input.deck_center_to_truck;
        let x = [1., 0., 0., 0.];
        let z = [0., 0., 1., 0.];
        Self {
            center: input.reference,
            far_points: [
                madd(x, distance, input.reference),
                sub(input.reference, scale(x, distance)),
            ],
            upmost_normal: z,
            direction: z,
            high_side: z,
            normal_limits: [0.; 2],
            kind: GeometryType::Impossible,
            audio_surface: 3,
            physics_surface: 1,
            flags: Self::INVALID,
            tilted_upmost_normal: orientation::tilted_normal(z, z, [0.; 2]),
        }
    }
}

/// 82C20728: six fixed descriptors and the optional seventh descriptor.
/// Returns None exactly for the native inadequate-up-vector gate.
pub fn prepare(input: InvestigationInput) -> Option<Investigation> {
    let delta = sub(input.end, input.start);
    let raw_up = cross(delta, cross(UP, delta));
    let up_length = length(raw_up);
    let retain_normalized = |v| {
        let square = dot(v, v);
        let inverse = inverse_length(square);
        let magnitude = if square == 0. { 0. } else { square * inverse };
        if magnitude > f32::from_bits(0x3586_37bd) {
            scale(v, inverse)
        } else {
            v
        }
    };
    let up = retain_normalized(raw_up);
    let direction = retain_normalized(delta);
    if up_length < f32::from_bits(0x3727_c5ac) {
        return None;
    }
    let projected = madd(
        direction,
        dot(sub(input.reference, input.start), direction),
        input.start,
    );
    let center = sub(input.reference, sub(input.reference, projected));
    let side = cross(up, direction);
    let short_up = scale(up, f32::from_bits(0x3d23_d70a));
    let near_side = scale(side, f32::from_bits(0x3db8_51ec));
    let far_side = scale(side, input.deck_center_to_truck);
    let far_up = scale(up, input.deck_center_to_truck * f32::from_bits(0x3f87_ae14));
    let raised = scale(up, f32::from_bits(0x3db8_51ec));
    let line = |at, half, radius| Probe {
        start: add(at, half),
        end: sub(at, half),
        radius,
    };
    let mut probes = vec![
        line(add(center, near_side), short_up, 0.),
        line(sub(center, near_side), short_up, 0.),
        line(add(center, far_side), far_up, 0.),
        line(sub(center, far_side), far_up, 0.),
        line(center, short_up, 0.),
        line(add(center, raised), near_side, f32::from_bits(0x3a83_126f)),
    ];
    if let Some(start) = input.optional_probe {
        probes.push(Probe {
            start,
            end: add(start, [0., -6., 0., 0.]),
            radius: 0.,
        });
    }
    Some(Investigation {
        center,
        upmost_normal: up,
        direction,
        probes,
    })
}

/// Full synchronous host equivalent of prepare/submit/wait/read/classify.
/// The callback must provide the original nearest-hit contract and packed
/// surface code for each probe, honoring its world-query filtering context.
pub fn investigate<E>(
    input: InvestigationInput,
    mut query: impl FnMut(usize, Probe) -> Result<Option<ProbeHit>, E>,
) -> Result<GrindSurface, E> {
    let Some(plan) = prepare(input) else {
        return Ok(GrindSurface::not_submitted(input));
    };
    let mut hits = [None; 7];
    for (index, &probe) in plan.probes.iter().enumerate() {
        hits[index] = query(index, probe)?;
    }
    Ok(classify::resolve(&plan, &hits, 0))
}

/// Explicit result processing also permits a native-style reused output flag
/// word. Bit29 is set on obstruction, not cleared on the other native branch.
pub fn resolve(
    plan: &Investigation,
    hits: &[Option<ProbeHit>; 7],
    previous_flags: u32,
) -> GrindSurface {
    classify::resolve(plan, hits, previous_flags)
}
