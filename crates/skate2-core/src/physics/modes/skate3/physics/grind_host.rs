//! Bind the manager to the current board materials and authored world queries.
use super::{
    grind_materials::GrindMaterials,
    player_input::grind::{self, Host, MaterialMode},
    settings::PhysicsSettings,
};
use skate3_core::{
    air::trajectory::grind_surface::{Probe, ProbeHit},
    physics::{board_runtime::BoardRuntime, board_world::BoardWorld, grind_contact::balance},
};

pub(super) struct LiveHost<'a> {
    pub board: &'a mut BoardRuntime,
    pub settings: &'a mut PhysicsSettings,
    pub materials: &'a GrindMaterials,
}

impl Host for LiveHost<'_> {
    fn apply_material_mode(&mut self, mode: MaterialMode) -> Result<(), String> {
        self.materials.apply(mode, self.board, self.settings);
        Ok(())
    }
    fn surface_probe(
        &mut self,
        world: &BoardWorld,
        actor: [u32; 2],
        index: usize,
        probe: Probe,
    ) -> Result<Option<ProbeHit>, String> {
        grind::world::surface_probe(world, actor, index, probe)
    }
    fn force_exit_line(
        &mut self,
        world: &BoardWorld,
        actor: [u32; 2],
        probe: balance::ForceExitProbe,
    ) -> Result<Option<balance::ForceExitHit>, String> {
        grind::world::force_exit_line(world, actor, probe)
    }
}
