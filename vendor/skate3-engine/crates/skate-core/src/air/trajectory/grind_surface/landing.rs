//! Original S3 TU3 82C21930 after six-query completion, not the S2 type3 gate.
//! Query ownership stays with the caller; None means actual failed completion.
use super::{GeometryType, GrindSurface, V, cross, dot, orientation, sub};

/// Native PossibleGrindOrientations output offsets. The returned up-vector is
/// separate: 82C21930 can leave it and these vectors untouched on failure.
#[derive(Clone, Copy, Debug)]
pub struct LandingOrientation {
    pub kind: GeometryType, // +0
    pub garbage: bool,      // +4
    pub boardslide_dir: V,  // +16
    pub tipslide_dir: V,    // +32
    pub backslash_dir: V,   // +48
    pub high_side: V,       // +64
}

/// 82C21930: primitive endpoints and grind_position form a six-probe geometry
/// investigation (optional_probe=None). Pass its completed surface as Some;
/// when prepare returns None, pass None and retain the caller's cached vectors.
/// takeoff_position is original r5, grind_position r6; neither is a velocity.
/// Completed kind3 is deliberately NOT rejected here; the analyzer owns its
/// later type3 rejection. Do not replace it with S2 82CFCA88's early type gate.
pub fn update_landing_orientation(
    surface: Option<&GrindSurface>,
    takeoff_position: V,
    grind_position: V,
    up: &mut V,
    out: &mut LandingOrientation,
) {
    let Some(surface) = surface else {
        out.kind = GeometryType::Impossible;
        out.garbage = true;
        return;
    };
    if surface.flags & GrindSurface::INVALID != 0 {
        // 82C21A10..82C21A5C: completed but invalid geometry has its own
        // fully written fallback, distinct from the no-completion branch.
        let x = [1., 0., 0., 0.];
        *up = [0., 1., 0., 0.];
        out.kind = GeometryType::ThinRail;
        out.garbage = true;
        out.boardslide_dir = x;
        out.tipslide_dir = x;
        out.backslash_dir = negate(x);
        out.high_side = x;
        return;
    }

    *up = orientation::tilted_normal(
        surface.direction,
        surface.upmost_normal,
        surface.normal_limits,
    );
    out.kind = surface.kind;
    out.garbage = false;
    out.high_side = surface.high_side;
    // Original cross products are not followed by normalization.
    let side = cross(surface.upmost_normal, surface.direction);
    let tipslide_angle = f32::from_bits(0x3eb2_b8c3); // 822F8618
    if surface.kind == GeometryType::ThinRail {
        out.boardslide_dir = side;
        let oriented = if dot(side, sub(takeoff_position, grind_position)) > 0. {
            side
        } else {
            negate(side)
        };
        let axis = cross(oriented, surface.upmost_normal);
        out.tipslide_dir = orientation::rotate(axis, oriented, tipslide_angle);
        out.backslash_dir = orientation::rotate(axis, oriented, f32::from_bits(0x401a_25c2)); // 822F93B8, stored pi-minus-backslash
    } else {
        let axis = cross(side, surface.upmost_normal);
        out.boardslide_dir = if surface.kind == GeometryType::FatRail {
            side
        } else {
            cross(*up, surface.direction)
        };
        let sign = if dot(side, surface.high_side) > 0. {
            1.
        } else {
            -1.
        };
        // Original sign vectors broadcast across all four lanes. These are
        // multiplies, unlike the XOR sign inversion in the thin-rail branch.
        out.backslash_dir = orientation::rotate(
            axis.map(|v| v * sign),
            side.map(|v| v * sign),
            f32::from_bits(0x3f3b_a866),
        ); // 822F8614
        out.tipslide_dir = orientation::rotate(
            axis.map(|v| v * -sign),
            side.map(|v| v * -sign),
            tipslide_angle,
        );
    }
}

/// Native XOR toggles every sign bit, including W and signed zero.
fn negate(v: V) -> V {
    v.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
}
