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
