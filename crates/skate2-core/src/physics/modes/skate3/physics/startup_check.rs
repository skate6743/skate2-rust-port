//! Map startup checks for `--validate-maps` (see crate::skate3_native::map_validation).
//! Mirrors the private `map_startup` test: spawn support, deck pose and a short
//! neutral startup must reach supported wheels with finite state. Results are
//! reported, not asserted.
use super::*;
use crate::skate3_native::{camera::CameraRuntime, graph_runtime::StockGraphs};
use skate3_core::input::gameplay_map::GameplayActions;

/// Neutral startup runs until the wheels stay supported for SETTLE_TICKS in a
/// row, up to STARTUP_TICKS (4 s at 60 Hz). Authored starts can sit up to ~1 m
/// above the floor (IndustrialSkatePark: 0.75 m), which a fixed 24-tick window
/// (the map_startup test's) does not always cover.
pub(crate) const STARTUP_TICKS: u64 = 240;
const SETTLE_TICKS: usize = 12;
/// Spawn support probe: from 1 m above the spawn to 10 m below it.
const SUPPORT_ABOVE: f32 = 1.0;
const SUPPORT_BELOW: f32 = 10.0;

pub(crate) struct StartupReport {
    /// Distance from the spawn down to the first supporting surface, if any.
    pub support_drop: Option<f32>,
    pub grounded_ticks: usize,
    pub warnings: Vec<String>,
}

pub(crate) fn collision_triangles(physics: &Skate3PhysicsState) -> usize {
    physics.world.triangles().len()
}

/// Spawn support, deck pose and a neutral startup on an already loaded map.
pub(crate) fn check(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &mut PlayerControls,
    camera: &mut CameraRuntime,
    graphs: &StockGraphs,
    spawn: [f32; 3],
    heading: f32,
) -> Result<StartupReport, String> {
    let mut warnings = Vec::new();
    let [x, y, z] = spawn;
    let support_drop = physics
        .world
        .query_thin_line(Vector3::new(x, y + SUPPORT_ABOVE, z), Vector3::new(x, y - SUPPORT_BELOW, z))
        .map_err(|e| format!("spawn support query: {e}"))?
        .map(|hit| y - hit.geometry.position.y);
    if support_drop.is_none() {
        warnings.push(format!("no collision within {SUPPORT_BELOW} m below the spawn"));
    }

    let deck = physics.board.part_transforms()[skate3_core::physics::board::BodyId::Deck.index()];
    if (deck.translation.x - x).abs() > 0.01 || (deck.translation.z - z).abs() > 0.01 {
        warnings.push(format!(
            "deck spawned at ({:.3}, {:.3}) instead of ({x:.3}, {z:.3})",
            deck.translation.x, deck.translation.z
        ));
    }
    let forward = deck.basis.columns[2];
    let (sin, cos) = heading.sin_cos();
    if (forward[0] - sin).abs() > 0.01 || (forward[2] - cos).abs() > 0.01 {
        warnings.push(format!("deck heading {forward:?} does not match map heading {heading}"));
    }

    let mut grounded_ticks = 0;
    let mut settled = 0;
    let mut solved_wheel_contact = false;
    for _ in 0..STARTUP_TICKS {
        if settled >= SETTLE_TICKS {
            break;
        }
        // No controller: the same zero actions the game publishes when no pad is ready.
        let mut actions = GameplayActions::from_values([0.0; 18]);
        controls.update_for_physics(&mut actions, physics, skater, camera)?;
        frame::advance(physics, skater, controls, graphs, &mut actions, true, camera)
            .map_err(|e| format!("tick {}: {e}", physics.ticks))?;
        if physics.failed {
            warnings.push(format!("physics failed at tick {}", physics.ticks));
            break;
        }
        let finite = physics
            .board
            .bodies()
            .iter()
            .chain(skater.skeleton.bodies())
            .all(|b| {
                [b.rates.position.x, b.rates.position.y, b.rates.position.z].iter().all(|v| v.is_finite())
                    && b.rates.basis.columns.iter().flatten().all(|v| v.is_finite())
            });
        if !finite {
            warnings.push(format!("non-finite body state at tick {}", physics.ticks));
            break;
        }
        let supported = physics.riding.ground.wheel_contact_count > 0;
        grounded_ticks += usize::from(supported);
        settled = if supported { settled + 1 } else { 0 };
        for report in physics.board.contact_reports() {
            let force = report.normal_force_on_a;
            solved_wheel_contact |= report.part.index() < 4
                && matches!(report.other, skate3_core::physics::board_step::CollisionBody::StaticWorld)
                && [force.x, force.y, force.z].iter().any(|v| *v != 0.0);
        }
    }
    if grounded_ticks == 0 {
        warnings.push(format!("wheels never found support in {STARTUP_TICKS} neutral ticks"));
    } else if settled < SETTLE_TICKS {
        warnings.push(format!("wheels did not stay supported for {SETTLE_TICKS} ticks within {STARTUP_TICKS} neutral ticks"));
    }
    if grounded_ticks > 0 && !solved_wheel_contact {
        warnings.push("no solved wheel contact reaction during startup".into());
    }
    Ok(StartupReport { support_drop, grounded_ticks, warnings })
}
