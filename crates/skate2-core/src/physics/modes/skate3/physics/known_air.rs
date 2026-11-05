//! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880.
//! All phase adapters borrow the production owners; the state copy below is a
//! stack-local value while those owners are borrowed, then published once.
mod flip;
mod math;
mod output;
mod runtime;
mod settings;
use super::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::air::known::{
    self, KnownAirFrame, KnownAirModeSettings, KnownAirOutput, KnownAirReckoningFields,
    KnownAirSettings, KnownAirState, KnownAirWipeoutRequest, KnownAirWipeoutSettings,
};
use skate_data::collections::Collections;
pub(crate) struct KnownAir {
    pub state: KnownAirState,
    settings: KnownAirSettings,
    modes: [KnownAirModeSettings; 5],
    wipeout: KnownAirWipeoutSettings,
    flip_axis_adjustment: [f32; 4],
}
impl KnownAir {
    pub(crate) fn adopt_mode_settings(&mut self, other: Self) { self.modes = other.modes; }
    pub fn load(data: &Collections) -> Result<Self, String> {
        let (settings, modes) = settings::load(data)?;
        Ok(Self {
            settings,
            modes,
            wipeout: KnownAirWipeoutSettings {
                air_falling_min_up_y_260: data.float(
                    "physics_wipeout",
                    "default",
                    "Wipeout_AirFallingMinUpY",
                )?,
                air_falling_max_angle_264: data.float(
                    "physics_wipeout",
                    "default",
                    "Wipeout_AirFallingMaxAngle",
                )?,
            },
            //82D8DA84 binds physics_reckoning handle1600, layout pointer1604.
            flip_axis_adjustment: data
                .words::<4>("physics_reckoning", "default", "FlipAxisAdjustment")?
                .map(f32::from_bits),
            state: KnownAirState {
                landing_normal_64: [0.0, 1.0, 0.0, 0.0],
                landing_heading_80: [0.0; 4],
                trajectory_apex_96: [0.0; 4],
                collision_position_112: [0.0; 4],
                selector_vector_128: [0.0; 4],
                trajectory_follow_offset_144: [0.0; 4],
                target_com_position_160: [0.0; 4],
                collision_normal_speed_176: 0.0,
                time_in_state_180: 0.0,
                start_y_184: 0.0,
                max_y_188: 0.0,
                com_max_y_192: 0.0,
                collision_time_196: 0.0,
                time_to_apex_200: 0.0,
                body_flip_target_speed_204: 0.0,
                reached_apex_208: false,
                landing_heading_valid_209: false,
                start_flipped_210: false,
                body_flipping_211: false,
                grind_air_adjust_activated_212: false,
                targeting_grind_213: false,
                trajectory_index_216: 0,
            },
        })
    }
    fn mode(&self, index: u32) -> Result<KnownAirModeSettings, String> {
        self.modes
            .get(index as usize)
            .copied()
            .ok_or_else(|| format!("Undefined KnownAir physics mode {index}"))
    }
}
fn frame(skater: &Skate3ControllerState, next: u32) -> Result<KnownAirFrame, String> {
    let p = &skater.player_input.processed;
    let t = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("KnownAir requires current BoardToolkit")?;
    let s = skater
        .trajectory
        .selector
        .selection()
        .ok_or("KnownAir requires a completed selected trajectory")?;
    Ok(KnownAirFrame {
        start_flip_reference_96: t.deck[2],
        alternate_head_target_112: t.deck[3],
        velocity_400: p.vectors_400_416[0].map(f32::from_bits),
        ground_normal_464: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
        start_height_484: f32::from_bits(p.vectors_464_480_496_512_528[1][1]),
        skater_up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        flags_2468: p.flags_2468,
        flags_2472: p.flags_2472,
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        flags_2488: p.flags_2488,
        next_physics_state_2500: next as i32,
        delta_time_2604: p.timestep_2604,
        forward_speed_2612: p.scalar_2612,
        body_spin_input_2640: skater.animation_input.fields.body_spin,
        selector_landing_normal_2656: s.landing_normal,
    })
}
fn reckoning(physics: &Skate3PhysicsState, skater: &Skate3ControllerState) -> KnownAirReckoningFields {
    let up = physics.riding.reckoning.up;
    KnownAirReckoningFields {
        landing_normal_1152: [up.x, up.y, up.z, 0.0],
        heading_axis_1200: physics.riding.reckoning_frames.heading,
        body_spin_angle_1568: skater.air_reckoning.state.spin_angle,
        body_spin_speed_1572: skater.air_reckoning.state.spin_speed,
    }
}
pub(super) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let frame = frame(skater, 201)?;
    let mut state = skater.known_air.state;
    let settings = skater.known_air.settings;
    let mode = skater
        .known_air
        .mode(skater.player_input.processed.state_variant_index_2528)?;
    let mut r = runtime::Runtime::new(physics, skater)?;
    known::enter(&mut state, &frame, &settings, &mode, &mut r);
    r.finish()?;
    skater.known_air.state = state;
    Ok(())
}
pub(super) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let frame = frame(skater, 201)?;
    let mut state = skater.known_air.state;
    let settings = skater.known_air.settings;
    let mode = skater
        .known_air
        .mode(skater.player_input.processed.state_variant_index_2528)?;
    let reckoning = reckoning(physics, skater);
    let mut r = runtime::Runtime::new(physics, skater)?;
    known::update(&mut state, &frame, &settings, &mode, &reckoning, &mut r);
    r.finish()?;
    skater.known_air.state = state;
    Ok(())
}
pub(super) fn exit(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    next: u32,
) -> Result<(), String> {
    let mut frame = frame(skater, next)?;
    let mut state = skater.known_air.state;
    let settings = skater.known_air.settings;
    let mut reckoning = reckoning(physics, skater);
    let mut r = runtime::Runtime::new(physics, skater)?;
    known::exit(&mut state, &mut frame, &settings, &mut reckoning, &mut r);
    r.finish()?;
    skater.player_input.processed.scalar_2612 = frame.forward_speed_2612;
    skater.air_reckoning.state.spin_angle = reckoning.body_spin_angle_1568;
    skater.air_reckoning.state.spin_speed = reckoning.body_spin_speed_1572;
    skater.known_air.state = state;
    Ok(())
}
///82D36590: apex observation, shared air-wipeout(true), then upside-down request.
pub(super) fn post_physics(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<(), String> {
    let frame = frame(skater, 201)?;
    let mut state = skater.known_air.state;
    let settings = skater.known_air.wipeout;
    let mode = skater
        .known_air
        .mode(skater.player_input.processed.state_variant_index_2528)?;
    let mut request = KnownAirWipeoutRequest {
        requested_35: false,
        scalar_116: 0.0,
        counter_200: 0,
    };
    let mut r = runtime::Runtime::new(physics, skater)?;
    known::update_post_physics(&mut state, &frame, &mode, &settings, &mut request, &mut r);
    r.finish()?;
    if request.counter_200 != 0 {
        skater.wipeout.state.request(15, request.scalar_116);
    }
    skater.known_air.state = state;
    Ok(())
}
pub(super) fn fill(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<KnownAirOutput, String> {
    let frame = frame(skater, 201)?;
    let state = skater.known_air.state;
    let mut out = output::storage(&skater.player_input.physical.air);
    let mut r = runtime::Runtime::new(physics, skater)?;
    known::fill_physics_output(&state, &frame, &mut out, &mut r);
    r.finish()?;
    output::publish(&out, &mut skater.player_input.physical.air);
    Ok(out)
}
