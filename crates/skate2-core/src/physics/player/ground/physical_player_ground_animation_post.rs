//! `GroundAnimation.Post82d83fb8` -> animated Ground detector82de1930.
//! Child of the existing same-pair Ground detector adapter.
use super::{
    Arc, DebugOverlaySlot, GroundWipeoutRuntimeError, GroundWipeoutSource, PhysicalAssemblyWorld,
    PhysicalPlayerAssemblyOwner, ReckoningNumeric, SimulationSkaterAssemblyHandle, Source,
    detector,
};

impl<N: ReckoningNumeric> Source<'_, N> {
    fn animated_opposing_delta(&mut self) -> Result<f32, GroundWipeoutRuntimeError<N::Error>> {
        self.contacts()?
            .lock()
            .map_err(|_| GroundWipeoutRuntimeError::Poisoned)?
            .collision_state
            .opposing_delta
            .ok_or(GroundWipeoutRuntimeError::Missing(
                "board opposing contact delta",
            ))
    }
    fn try_animated_ground_wipeout(&mut self) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        detector::try_check_ground(self)?;
        let arms_pointer = self.air_attribute_pointer(0xd031_7181_ce40_bfbc)?;
        let body_pointer = self.air_attribute_pointer(0x68e0_e915_d4f3_af66)?;
        let arms = self.wipeout_float(&arms_pointer)?;
        let body = self.wipeout_float(&body_pointer)?;
        if detector::leaves::contact_force(self, body, arms)? {
            self.request(0)?;
            if self.air_display_debug()? {
                self.air_require_overlay()?;
            }
        }
        let delta = self.animated_opposing_delta()?;
        let threshold = self.air_attribute(0x5d30_8c99_bc02_32e2)?;
        // Raw1b30 BO4/BI25: GT-clear skips, including unordered operands.
        if delta > threshold {
            if self.air_display_debug()? {
                let _fresh_delta = self.animated_opposing_delta()?;
                self.air_require_overlay()?;
            }
            self.request(20)?;
        }
        Ok(())
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_ground_animation_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), GroundWipeoutRuntimeError<N::Error>> {
        use GroundWipeoutRuntimeError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        Source {
            world: self,
            board,
            skeleton,
            owner,
            numeric,
            overlay,
        }
        .try_animated_ground_wipeout()
    }
}
