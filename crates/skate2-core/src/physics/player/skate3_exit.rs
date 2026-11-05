//! One-time native solver handoff. Actor reset generations and physical body
//! occurrences prevent a mode change from affecting a later marker/career reset.
use crate::{
    actor_construction::ActorConstruction,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
};

pub(crate) struct PlayerExit {
    pub generation: u32,
    pub rates: Vec<BodyRate>,
}
#[derive(Clone, Copy)]
pub(crate) struct BodyRate {
    pub body: SimulationRigidBodyObservation,
    pub centre: [f32; 3],
    pub velocity: [f32; 3],
    pub omega: [f32; 3],
}
impl BodyRate {
    pub(crate) fn at(&self, target: [f32; 3]) -> Result<[f32; 3], String> {
        if self
            .centre
            .iter()
            .chain(&self.velocity)
            .chain(&self.omega)
            .chain(&target)
            .any(|value| !value.is_finite())
        {
            return Err("Nonfinite native exit rate".into());
        }
        let r: [f32; 3] = std::array::from_fn(|i| target[i] - self.centre[i]);
        let w = self.omega;
        let result = [
            self.velocity[0] + w[1] * r[2] - w[2] * r[1],
            self.velocity[1] + w[2] * r[0] - w[0] * r[2],
            self.velocity[2] + w[0] * r[1] - w[1] * r[0],
        ];
        if result.iter().any(|value| !value.is_finite()) {
            return Err("Overflow in native exit rate".into());
        }
        Ok(result)
    }
}
pub(crate) fn apply(
    world: &mut PhysicalAssemblyWorld,
    owner: &PhysicalPlayerAssemblyOwner,
    actor: &ActorConstruction,
) -> Result<(), String> {
    let exit = owner
        .skate3_exit
        .lock()
        .map_err(|_| "native exit lock")?
        .take();
    if let Some(exit) = exit {
        let actual = actor
            .try_teleport_publication()
            .map_err(|error| error.to_string())?
            .0;
        if actual != exit.generation {
            crate::diagnostics::test_log::event(
                "skate3_exit_cancelled",
                format_args!("expected={} actual={actual}", exit.generation),
            );
            return Ok(());
        }
        world.backend_mut().try_apply_mode_exit_rates(&exit.rates)?;
        crate::diagnostics::test_log::event(
            "skate3_exit_applied",
            format_args!("generation={actual} bodies={}", exit.rates.len()),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        simulation_rigid_body_pool::SimulationRigidBodyPool,
        skateboard_wheels::FRESH_VOLUME_TRANSFORM as IDENTITY,
    };
    #[test]
    fn skate3_exit_rejects_invalid_or_foreign_rates_before_any_native_store() {
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let first = pool.try_add_rigid_body(IDENTITY, None, 4).unwrap().unwrap();
        let second = pool.try_add_rigid_body(IDENTITY, None, 4).unwrap().unwrap();
        let first = pool.try_observe(first).unwrap();
        let second = pool.try_observe(second).unwrap();
        let before = pool.try_drive_build_snapshot(first).unwrap().velocity;
        let rate = BodyRate {
            body: first,
            centre: [0.; 3],
            velocity: [2., 3., 4.],
            omega: [0., 1., 0.],
        };
        let bad = BodyRate {
            body: second,
            velocity: [f32::NAN, 0., 0.],
            ..rate
        };
        assert!(pool.try_apply_mode_exit_rates(&[rate, bad]).is_err());
        assert_eq!(
            pool.try_drive_build_snapshot(first).unwrap().velocity,
            before
        );
        let mut other = SimulationRigidBodyPool::try_new(1).unwrap();
        let body = other
            .try_add_rigid_body(IDENTITY, None, 4)
            .unwrap()
            .unwrap();
        let foreign = BodyRate {
            body: other.try_observe(body).unwrap(),
            ..rate
        };
        assert!(pool.try_apply_mode_exit_rates(&[rate, foreign]).is_err());
        assert_eq!(
            pool.try_drive_build_snapshot(first).unwrap().velocity,
            before
        );
        pool.try_set_externally_driven(&[first, second]).unwrap();
        assert!(pool.try_return_external_player().is_err());
        assert!(
            pool.is_externally_driven(first),
            "failed return changed ownership"
        );
        let mut moved = IDENTITY;
        moved[12] = 10.;
        pool.try_publish_mode_body_views(&[
            (first, moved, [2., 3., 4.], [0., 1., 0.]),
            (second, IDENTITY, [0.; 3], [0.; 3]),
        ])
        .unwrap();
        pool.try_return_external_player().unwrap();
        assert!(!pool.is_externally_driven(first));
        let returned = pool.try_drive_build_snapshot(first).unwrap();
        assert_eq!(returned.center_of_mass, [10., 0., 0.]);
        assert_eq!(returned.velocity, [2., 3., 4.]);
        assert_eq!(pool.frame_identity(), None);
        pool.try_apply_mode_exit_rates(&[rate]).unwrap();
        assert_eq!(
            pool.try_drive_build_snapshot(first).unwrap().velocity,
            [2., 3., -6.]
        );
    }
}
