//! Attached Body::AddLocalForce82de8518 and AddTorque82de8358, in the
//! WORLD-force/BODY-position arms used by DynamicObject82d2a4f8/82d2a588.
//! The caller resolves the original Part 0 and establishes a nonnull island.
use super::{
    SimulationRigidBodyObservation, SimulationRigidBodyPool, SimulationRigidBodyPoolError,
    skateboard_force::cached_tensor_product,
};
use crate::{
    reckoning_numeric::cross_full4,
    skeleton_collision_numeric::{HostSkeletonCollisionNumeric, try_clamp_xyz},
    skeleton_root_transforms::SkeletonRootVector,
};

impl SimulationRigidBodyPool {
    fn try_wake_grab_body(
        &mut self,
        observation: SimulationRigidBodyObservation,
    ) -> Result<usize, SimulationRigidBodyPoolError> {
        let index = self.validate_observation(observation)?;
        self.validate_integrity()?;
        if self.slots[index].state & 7 == 2 {
            self.try_activate_frozen_body(observation)?;
        }
        self.slots[index].cooldown = 0;
        Ok(index)
    }

    /// Position is a BODY-space lever arm, transformed by orientation only.
    /// There is no translation/subtraction of the world centre in this arm.
    pub(crate) fn try_apply_grab_local_force(
        &mut self,
        observation: SimulationRigidBodyObservation,
        force: SkeletonRootVector,
        position: SkeletonRootVector,
        angular_acceleration_limit: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.try_wake_grab_body(observation)?;
        let slot = &mut self.slots[index];
        let first = slot.right.map(|v| v * position.xyz[0]);
        let second: [f32; 3] =
            std::array::from_fn(|i| slot.up[i].mul_add(position.xyz[1], first[i]));
        let arm: [f32; 3] = std::array::from_fn(|i| slot.at[i].mul_add(position.xyz[2], second[i]));
        // W does not feed XYZ, and none of the packed W/link words is stored.
        let moment = cross_full4(
            [arm[0], arm[1], arm[2], 0.0],
            [force.xyz[0], force.xyz[1], force.xyz[2], 0.0],
        );
        let acceleration = force.xyz.map(|v| v * slot.inverse_mass);
        let old = slot.force;
        let z = old[2] + acceleration[2];
        slot.force[0] = acceleration[0] + old[0];
        slot.force[1] = old[1] + acceleration[1];
        slot.force[2] = z;
        if std::env::var_os("SKATE2_TRACE_PUSHING_MOVE").is_some() {
            eprintln!("grab force: body slot {index} inverse mass {} force accumulator {:?}", slot.inverse_mass, slot.force);
        }
        let angular = cached_tensor_product(
            slot.inverse_inertia_full,
            slot.inverse_inertia_split,
            moment,
        );
        let old = slot.torque;
        let y = angular[1] + old[1];
        slot.torque[0] = old[0] + angular[0];
        slot.torque[1] = y;
        slot.torque[2] = angular[2] + old[2];
        slot.cooldown = 0;
        clamp_accumulated_torque(&mut slot.torque, angular_acceleration_limit);
        Ok(())
    }

    pub(crate) fn try_apply_grab_world_torque(
        &mut self,
        observation: SimulationRigidBodyObservation,
        torque: SkeletonRootVector,
        angular_acceleration_limit: f32,
    ) -> Result<(), SimulationRigidBodyPoolError> {
        let index = self.try_wake_grab_body(observation)?;
        let slot = &mut self.slots[index];
        let angular = cached_tensor_product(
            slot.inverse_inertia_full,
            slot.inverse_inertia_split,
            [torque.xyz[0], torque.xyz[1], torque.xyz[2], 0.0],
        );
        let old = slot.torque;
        let z = old[2] + angular[2];
        slot.torque[1] = old[1] + angular[1];
        slot.torque[0] = old[0] + angular[0];
        slot.torque[2] = z;
        slot.cooldown = 0;
        clamp_accumulated_torque(&mut slot.torque, angular_acceleration_limit);
        Ok(())
    }
}

fn clamp_accumulated_torque(torque: &mut [f32; 3], maximum: f32) {
    // Both Body routines clamp the entire accumulator, after storing it.
    // Ordered >0 deliberately skips zero, negative and NaN authored limits.
    if maximum > 0.0 {
        *torque = try_clamp_xyz(*torque, maximum, &mut HostSkeletonCollisionNumeric)
            .expect("positive maximum and infallible host clamp numeric");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;

    fn fixture(state: u32) -> (SimulationRigidBodyPool, SimulationRigidBodyObservation) {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let handle = pool
            .try_add_rigid_body(FRESH_VOLUME_TRANSFORM, None, state)
            .unwrap()
            .unwrap();
        let body = pool.try_observe(handle).unwrap();
        let slot = &mut pool.slots[0];
        slot.inverse_mass = 0.25;
        slot.inverse_inertia_full = [2.0, 3.0, 5.0];
        slot.inverse_inertia_split = [11.0, 7.0, 13.0];
        (pool, body)
    }

    #[test]
    fn body_space_arm_uses_existing_pose_tensor_and_wakes_actual_frozen_occurrence() {
        let (mut pool, body) = fixture(2);
        let slot = &mut pool.slots[0];
        slot.right = [0.0, 1.0, 0.0];
        slot.up = [-1.0, 0.0, 0.0];
        slot.center_of_mass = [1000.0, -2000.0, 3000.0];
        slot.force = [7.0, 11.0, 13.0];
        slot.torque = [17.0, 19.0, 23.0];
        let velocity = slot.velocity;
        let omega = slot.omega;
        pool.try_apply_grab_local_force(
            body,
            SkeletonRootVector::known([2.0, -3.0, 5.0, f32::NAN]),
            SkeletonRootVector::known([1.0, 2.0, 3.0, f32::NAN]),
            0.0,
        )
        .unwrap();
        let slot = &pool.slots[0];
        // Rotated arm [-2,1,3]; arm x force [14,16,4]. Cached tensor
        // product [96,206,322], added to the existing torque accumulator.
        assert_eq!(slot.force, [7.5, 10.25, 14.25]);
        assert_eq!(slot.torque, [113.0, 225.0, 345.0]);
        assert_eq!(slot.velocity, velocity);
        assert_eq!(slot.omega, omega);
        assert_eq!(slot.state & 7, 4);
        assert_eq!(slot.cooldown, 0);
        assert_eq!(pool.counts.frozen, 0);
        assert_eq!(pool.counts.active, 1);
        pool.validate_integrity().unwrap();
    }

    #[test]
    fn even_zero_added_torque_wakes_and_clamps_the_whole_accumulator() {
        let (mut pool, body) = fixture(2);
        pool.slots[0].torque = [3.0, 4.0, 0.0];
        let force = pool.slots[0].force;
        pool.try_apply_grab_world_torque(body, SkeletonRootVector::known([0.0; 4]), 2.5)
            .unwrap();
        for (actual, expected) in pool.slots[0].torque.into_iter().zip([1.5, 2.0, 0.0]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(pool.slots[0].force, force);
        assert_eq!(pool.slots[0].state & 7, 4);
        assert_eq!(pool.slots[0].cooldown, 0);
    }

    #[test]
    fn world_torque_ignores_body_orientation_and_nonpositive_or_nan_limit() {
        for limit in [0.0, -1.0, f32::NAN] {
            let (mut pool, body) = fixture(4);
            pool.slots[0].right = [0.0, 1.0, 0.0];
            pool.slots[0].up = [-1.0, 0.0, 0.0];
            pool.slots[0].torque = [17.0, 19.0, 23.0];
            pool.try_apply_grab_world_torque(
                body,
                SkeletonRootVector::known([2.0, -3.0, 5.0, 0.0]),
                limit,
            )
            .unwrap();
            // Cached world tensor*[2,-3,5] = [20,50,26].
            assert_eq!(pool.slots[0].torque, [37.0, 69.0, 49.0]);
        }
    }

    #[test]
    fn foreign_observation_cannot_wake_or_accumulate_on_another_pool() {
        let (_, foreign) = fixture(2);
        let (mut pool, _) = fixture(2);
        let before = pool.snapshot();
        assert_eq!(
            pool.try_apply_grab_world_torque(foreign, SkeletonRootVector::known([1.0; 4]), 2.5,),
            Err(SimulationRigidBodyPoolError::ForeignHandle)
        );
        assert_eq!(pool.snapshot(), before);
    }
}
