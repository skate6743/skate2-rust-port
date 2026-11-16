//! Locate invalid physical values before they propagate into a rendered pose.
use super::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::physics::assembly::BodySnapshot;

pub(super) fn snapshot(physics: &Skate3PhysicsState, skater: &Skate3ControllerState) -> Vec<BodySnapshot> {
    physics.board.bodies().iter()
        .chain(std::iter::once(&physics.board.hook().body))
        .chain(skater.skeleton.bodies())
        .chain(skater.skeleton_drives.targets.bodies.iter())
        .copied().collect()
}

/// SKATE2_DIAGNOSTIC_SOLVER_FAULT=<tick>:<board|skeleton|both|explode>: on that tick a NaN in the
/// solved deck or hips position, or a deck leaving the solve at 30 m/s upward, to exercise the
/// game's answers to a solver result it rejects.
pub(super) fn inject_fault(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    static FAULT: std::sync::OnceLock<Option<(u64, String)>> = std::sync::OnceLock::new();
    let Some((tick, part)) = FAULT.get_or_init(|| {
        let value = std::env::var("SKATE2_DIAGNOSTIC_SOLVER_FAULT").ok()?;
        let (tick, part) = value.split_once(':')?;
        Some((tick.parse().ok()?, part.to_owned()))
    }) else {
        return;
    };
    if physics.ticks != *tick {
        return;
    }
    if matches!(part.as_str(), "board" | "both") {
        physics.board.bodies_mut()[6].rates.position.x = f32::NAN;
    }
    if matches!(part.as_str(), "skeleton" | "both") {
        skater.skeleton.bodies_mut()[23].rates.position.x = f32::NAN;
    }
    if part == "explode" {
        physics.board.bodies_mut()[6].rates.linear_velocity.y = 30.0;
    }
}

pub(super) fn validate(bodies: &[BodySnapshot], stage: &str) -> Result<(), String> {
    for (index, body) in bodies.iter().enumerate() {
        let r = &body.rates;
        for (name, v) in [
            ("position", r.position),
            ("linear_velocity", r.linear_velocity),
            ("angular_velocity", r.angular_velocity),
            ("force_acceleration", r.force_acceleration),
            ("torque_acceleration", r.torque_acceleration),
        ] {
            if ![v.x, v.y, v.z].into_iter().all(f32::is_finite) {
                return Err(format!("Non-finite {name} {stage}; reaction_body={index}; body={body:?}"));
            }
        }
        let q = r.orientation;
        if ![q.x, q.y, q.z, q.w].into_iter().all(f32::is_finite)
            || !r.basis.columns.iter().flatten().all(|v| v.is_finite())
            || !r.world_inverse_inertia.columns.iter().flatten().all(|v| v.is_finite()) {
            return Err(format!("Non-finite orientation/inertia {stage}; reaction_body={index}; body={body:?}"));
        }
    }
    Ok(())
}
