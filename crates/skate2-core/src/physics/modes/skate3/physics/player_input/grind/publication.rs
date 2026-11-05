use super::*;
use crate::skate3_native::physics::grind::{Family, observation::*};

pub(super) fn surface(fields: &mut GrindInvestigationFields, surface: &GrindSurface) {
    fields.center_1360 = raw(surface.center);
    fields.far_points_1376_1392 = surface.far_points.map(raw);
    fields.upmost_normal_1408 = raw(surface.upmost_normal);
    fields.surface_direction_1424 = raw(surface.direction);
    fields.high_side_1440 = raw(surface.high_side);
    fields.normal_limits_1456_1460 = surface.normal_limits;
    fields.geometry_kind_1464 = surface.kind as u32;
    fields.audio_surface_1468 = surface.audio_surface;
    fields.physics_surface_1472 = surface.physics_surface;
    fields.geometry_flags_1476 = surface.flags;
}

pub(super) fn observation(
    p: &ProcessedPhysicsInput,
    c: PostContext,
    metadata: Option<PrimitiveMetadata>,
    jumper: &manager::Jumper,
) -> Result<ManagerObservation, String> {
    let f = &p.grind;
    family(f.family_1248)?;
    family(jumper.family)?;
    Ok(ManagerObservation {
        geometry: GeometryObservation {
            point_1120: float(f.point_1120),
            direction_1136: float(f.direction_1136),
            normal_1152: float(f.normal_1152),
            target_up_1168: float(f.target_up_1168),
            primitive_start_1264: float(f.primitive_start_1264),
            primitive_end_1280: float(f.primitive_end_1280),
            spline_guids_1296: metadata.and_then(|m| m.spline_guids),
            upmost_normal_1408: float(f.upmost_normal_1408),
            high_side_1440: float(f.high_side_1440),
            kind_1464: f.geometry_kind_1464,
            flags_1476: f.geometry_flags_1476,
            impact_speed_1492: f.impact_speed_1492,
        },
        surface: SurfaceObservation {
            audio_surface_1468: f.audio_surface_1468,
            material_1472: f.physics_surface_1472,
            friction_vs_time_1496: f.friction_1496,
            reckon_blend_selector_1500: f.exit_lean_1500,
            gravity_relief_1512: f.gravity_relief_1512,
        },
        control: ControlObservation {
            flags_1516: f.flags_1516,
            flags_2488: p.flags_2488,
            translation_2796: c.translation_2796,
            balance_2800: c.stability_nudge_2800,
        },
        engagement: EngagementObservation {
            kind_1248: f.family_1248,
        },
    })
}

fn family(value: u32) -> Result<Family, String> {
    Ok(match value {
        0 => Family::FiftyFifty,
        1 => Family::Boardslide,
        2 => Family::Tipslide,
        3 => Family::FiveO,
        4 => Family::Backslash,
        5 => Family::Darkslide,
        _ => return Err(format!("Invalid published grind family {value}")),
    })
}
