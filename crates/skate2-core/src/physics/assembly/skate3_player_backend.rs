//! Narrow access to the retained body pool for the explicitly selected PC mod.
use super::SimulationAssemblyBackend;
use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyDriveBuildSnapshot, SimulationRigidBodyObservation,
};

impl<M, JF, JL> SimulationAssemblyBackend<M, JF, JL> {
    pub(crate) fn try_select_external_player(
        &mut self,
        bodies: &[SimulationRigidBodyObservation],
    ) -> Result<(), String> {
        self.rigid_bodies
            .try_set_externally_driven(bodies)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn try_external_body_snapshot(
        &self,
        body: SimulationRigidBodyObservation,
    ) -> Result<SimulationRigidBodyDriveBuildSnapshot, String> {
        self.rigid_bodies
            .try_drive_build_snapshot(body)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn try_return_external_player(&mut self) -> Result<(), String> {
        self.rigid_bodies.try_return_external_player()
    }
    pub(crate) fn try_apply_mode_exit_rates(
        &mut self,
        rates: &[crate::skate3_native::physics::player::exit::BodyRate],
    ) -> Result<(), String> {
        self.rigid_bodies.try_apply_mode_exit_rates(rates)
    }
    pub(crate) fn try_publish_mode_body_views(
        &mut self,
        values: &[(SimulationRigidBodyObservation, [f32;16], [f32;3], [f32;3])],
    ) -> Result<(), String> {
        self.rigid_bodies.try_publish_mode_body_views(values)
    }
}
