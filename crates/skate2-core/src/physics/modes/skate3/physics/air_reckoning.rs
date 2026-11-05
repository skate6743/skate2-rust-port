//! Production stock settings and live-input adapter for Reckoning82D8DBD8.
mod settings;
use super::riding_outputs::RidingOutputs;
use skate3_core::{
    air::{
        reckoning::{self, AirState, Input, Settings},
        state::PhysicsAirReckoningFields,
    },
    player::input_phase::ProcessedPhysicsInput,
};
use skate_data::collections::Collections;

/// The same mandatory source settings used by the full S3 player.
pub(crate) fn load_assist_settings(data: &Collections) -> Result<Settings, String> {
    settings::load(data).map(|(settings, _)| settings)
}

pub(crate) struct AirReckoning {
    pub state: AirState,
    settings: Settings,
    modes: [settings::Mode; 5],
}
impl AirReckoning {
    pub(crate) fn adopt_mode_settings(&mut self, other: Self) { self.modes = other.modes; }
    pub fn update_plant(&mut self,riding:&mut RidingOutputs,p:&ProcessedPhysicsInput,up:[f32;4],heading:[f32;4]) {
        reckoning::update_plant(&mut riding.reckoning,&mut riding.reckoning_frames,
            &mut riding.body_spin,&mut self.state,&self.settings,up,heading,p.flags_2468&(1<<20)!=0);
    }
    pub fn load(data: &Collections) -> Result<Self, String> {
        let (settings, modes) = settings::load(data)?;
        Ok(Self {
            state: AirState::new(),
            settings,
            modes,
        })
    }
    ///The Ground/PhysicsAir lifecycles borrow this canonical pair for their
    ///native spin resets. Filters and matrix1008 remain owned by RidingOutputs.
    pub fn fields(&self, riding: &RidingOutputs) -> PhysicsAirReckoningFields {
        let up = riding.reckoning.up;
        let normal = riding.reckoning.ground_normal;
        PhysicsAirReckoningFields {
            current_landing_normal_1152: [up.x, up.y, up.z, 0.0],
            collision_reference_normal_1216: [normal.x, normal.y, normal.z, 0.0],
            body_spin_angle_1568: self.state.spin_angle,
            body_spin_speed_1572: self.state.spin_speed,
        }
    }
    ///Called by the actual PhysicsAir/KnownAir update before its Board phase.
    ///Return the newly published normal1216, not the caller's earlier snapshot.
    pub fn update(
        &mut self,
        riding: &mut RidingOutputs,
        processed: &ProcessedPhysicsInput,
        physical_body_spin: f32,
        landing_normal: [f32; 4],
        normal_blend: f32,
        target_spin: f32,
        flip_request: f32,
    ) -> Result<PhysicsAirReckoningFields, String> {
        let mode = self
            .modes
            .get(processed.state_variant_index_2528 as usize)
            .ok_or_else(|| {
                format!(
                    "Undefined physics mode {}",
                    processed.state_variant_index_2528
                )
            })?;
        reckoning::update(
            &mut riding.reckoning,
            &mut riding.reckoning_frames,
            &mut riding.body_spin,
            &mut self.state,
            &self.settings,
            &Input {
                landing_normal,
                normal_blend,
                target_spin,
                flip_request,
                com_to_deck: processed.animation_com_to_deck_752.map(f32::from_bits),
                timestep: processed.timestep_2604,
                physical_body_spin,
                grind_adjusted_body_spin: processed.grind_adjusted_body_spin_2644,
                additive_spin: processed.flags_2484 & (1 << 15) != 0,
                direct_spin: processed.flags_2472 & (1 << 28) != 0,
                reverse_stance: processed.flags_2468 & (1 << 20) != 0,
                easy_body_spins: mode.easy_body_spins,
                perfect_body_flips: mode.perfect_body_flips,
            },
        );
        Ok(self.fields(riding))
    }
}
