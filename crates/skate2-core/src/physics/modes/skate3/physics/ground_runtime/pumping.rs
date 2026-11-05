//! Actual Ground82D37D34..7EAC pumping preparation and stock bindings.
use super::super::{animated_skeleton::AnimatedSkeleton, riding_outputs::RidingOutputs};
use skate3_core::{
    math::Vector3,
    physics::board_toolkit::BoardToolkit,
    riding::{
        collision_response::signed_angle,
        ground_correction_math::dot_product as dot3,
        pumping::{
            controller::{self, PumpingSample},
            geometry::NativePumpingGeometry,
            settings::{PumpingMode, PumpingSettings},
            state::PumpingState,
        },
    },
};
use skate_data::collections::Collections;

pub(crate) struct GroundPumping {
    settings: PumpingSettings,
    modes: [GroundPumpingMode; 5],
}
#[derive(Clone, Copy)]
pub(crate) struct GroundPumpingMode {
    controller: PumpingMode,
    /// Selected physics_mode layout+8, consumed by CalcPumpForce82D933F0.
    pub unintentional_scalar: f32,
}
impl GroundPumping {
    ///Revert82D435FC..36A0 uses the processed timestep and flag2472 bit16.
    pub fn update_revert(
        &self,
        state: &mut PumpingState,
        toolkit: &BoardToolkit,
        riding: &RidingOutputs,
        skeleton: &AnimatedSkeleton,
        p: &skate3_core::player::input_phase::ProcessedPhysicsInput,
        balance: f32,
    ) -> Result<(), String> {
        let angle = if balance != 0.0 {
            0.0
        } else {
            (skate3_core::physics::board_ground::angle_between(
                xyz(toolkit.deck[2]),
                riding.reckoning.up,
            )
            .abs()
                - std::f32::consts::FRAC_PI_2)
                .abs()
        };
        let sample = PumpingSample {
            position: toolkit.deck[3],
            normal: v(riding.reckoning.ground_normal),
            com_to_deck_world: skeleton.record.com_to_deck_world,
            deck_angle: angle,
            intentional_pumping: ((p.flags_2472 >> 16) & 1) as u8,
        };
        match controller::update(
            state,
            &self.settings,
            self.mode(p.state_variant_index_2528)?.controller,
            sample,
            p.timestep_2604,
            &mut NativePumpingGeometry,
        ) {
            Ok(()) => Ok(()),
            Err(never) => match never {},
        }
    }
    pub fn load(data: &Collections) -> Result<Self, String> {
        let f = |field| data.float("physics_pumping", "default", field);
        let c = |field| super::settings::curve8(data, "physics_pumping", "default", field);
        let mode = |key| -> Result<GroundPumpingMode, String> {
            let m = |field| data.float("physics_mode", key, field);
            Ok(GroundPumpingMode {
                controller: PumpingMode {
                    maximum_absorption_per_second: m("Hash_9D1AEE3D7D8A4A7")?,
                    maximum_acceleration_per_second: m("Hash_D77AFD320B6241C5")?,
                    absorption_factor: m("PumpEffectFactorAbsorption")?,
                    acceleration_factor: m("PumpEffectFactor")?,
                },
                unintentional_scalar: m("UnintentionalPumpScalar")?,
            })
        };
        Ok(Self {
            settings: PumpingSettings {
                pump_vs_speed: c("PumpVsVel")?,
                pump_vs_time: c("PumpVsTime")?,
                min_crouch_vs_ground_angle: c("MinCrouchVsGroundAngle")?,
                compression_vs_ground_angle: c("CompressionVsGroundAngle")?,
                compression_vs_deck_angle: c("CompressionVsDeckAngle")?,
                height_change_damping: f("PumpEffectDamping")?,
                minimum_height_change: f("MinChangeInCOMBeforePumping")?,
                maximum_height_change: f("MaxDeltaHeightAllowedPerFrame")?,
                ground_compression_scale: f("CompressionGroundScalar")?,
                deck_compression_scale: f("CompressionDeckScalar")?,
                angular_speed_damping: f("AngularSpeedDamping")?,
            },
            modes: [
                mode("easy")?,
                mode("normal")?,
                mode("hardcore")?,
                mode("motorized")?,
                mode("test")?,
            ],
        })
    }
    /// Ground82D37E74 reads the current Processed2548 mode, not the startup mode.
    pub fn mode(&self, index: u32) -> Result<GroundPumpingMode, String> {
        self.modes
            .get(index as usize)
            .copied()
            .ok_or_else(|| format!("Invalid pumping physics mode {index}"))
    }
    ///Runs after contact-state update and before Ground::UpdateSkateboard.
    pub fn update(
        &self,
        state: &mut PumpingState,
        toolkit: &BoardToolkit,
        riding: &RidingOutputs,
        skeleton: &AnimatedSkeleton,
        flags_2476: u32,
        mode: GroundPumpingMode,
    ) {
        let up = v(riding.reckoning.up);
        let animated_at = skeleton.board_frames.animation_target[2];
        let deck_angle = if dot3(toolkit.deck[2], up).abs() > dot3(animated_at, up).abs() {
            let axis = xyz(riding.reckoning_frames.ground[0]);
            let deck = wrap(signed_angle(xyz(toolkit.deck[2]), xyz(up), axis));
            let animated = wrap(signed_angle(xyz(animated_at), xyz(up), axis));
            (deck - animated).abs()
        } else {
            0.0
        };
        let sample = PumpingSample {
            position: toolkit.deck[3],
            normal: v(riding.reckoning.ground_normal),
            com_to_deck_world: skeleton.record.com_to_deck_world,
            deck_angle,
            intentional_pumping: ((flags_2476 >> 1) & 1) as u8,
        };
        match controller::update_ground(
            state,
            &self.settings,
            mode.controller,
            sample,
            &mut NativePumpingGeometry,
        ) {
            Ok(()) => {}
            Err(never) => match never {},
        }
    }
}
fn wrap(angle: f32) -> f32 {
    let turns = angle * f32::from_bits(0x3e22_f983);
    let fraction = turns - turns.floor();
    let signed = fraction - if fraction > 0.5 { 1.0 } else { 0.0 };
    signed * f32::from_bits(0x40c9_0fdb)
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn v(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
