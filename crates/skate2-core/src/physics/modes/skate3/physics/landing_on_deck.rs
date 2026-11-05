//! State503 lifecycle from original TU3 82D4D418..82D4E100.
mod publication;
mod skeleton;
use super::{Skate3PhysicsState, Skate3ControllerState};
pub(crate) use publication::fill;
use skate3_core::{
    physics::{
        skeleton_animation_record::compose_affine,
        skeleton_landing_on_board::Settings as RootSettings,
    },
    player::{
        offboard::{
            board_possession::lifecycle::Effects as _,
            landing_state::{Entry, Settings, State},
        },
        wipeout::Frame,
    },
};
use skate_data::collections::Collections;
pub(crate) struct Runtime {
    pub state: State,
    settings: Settings,
    root_settings: RootSettings,
}
impl Runtime {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let get = |name| data.float("physics_landingondeck", "default", name);
        Ok(Self {
            state: State::default(),
            settings: Settings {
                minimum_auto_angle: get("MinAngleToAutoTurn")?,
                automatic_speed: get("BodySpinSpeedAuto")?,
                input_speed: get("BodySpinSpeed")?,
                input_delta: get("BodySpinDeltaInput")?,
                automatic_delta: get("BodySpinDeltaAuto")?,
                maximum_landing_speed: data.float(
                    "physics_wipeout",
                    "default",
                    "MaxSpeedLandingOnBoard",
                )?,
            },
            root_settings: RootSettings {
                root_y_offset: data.float("physics_skeleton", "default", "SkateRootYOffset")?,
                capsule_radius: data.float(
                    "physics_skeleton",
                    "default",
                    "SkateRootCapsuleRadius",
                )?,
                capsule_length: data.float(
                    "physics_skeleton",
                    "default",
                    "SkateRootCapsuleLength",
                )?,
            },
        })
    }
}
fn clear_ik(skater: &mut Skate3ControllerState) {
    for limb in &mut skater.foot_ik.state.limbs {
        limb.board_blend = 0.;
        limb.external_blend = 0.;
        limb.mode = skate3_core::animation::foot_ik::status::Mode::Disabled;
    }
    skater.foot_ik.enable_feet(false);
}
pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater
        .ground_lifecycle
        .skeleton_controller
        .request(6, &mut skater.skeleton_collision)?;
    let p = skater.player_input.processed;
    let board = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Landing entry requires the completed board toolkit")?
        .deck;
    let hips = compose_affine(
        &skater.animated_skeleton.roots.animation_to_world,
        &skater.animated_skeleton.record.pose[23],
    );
    skater.landing_on_deck.state.enter(
        &mut skater.landing_deck.manager,
        Entry {
            previous_category: p.category_2516,
            hippy: p.flags_2480 & 0x1000 != 0,
            strength: skater.animation_input.extra.jump_strength,
            board_position: board[3],
            com_position: p.vectors_544_560_592_608[2].map(f32::from_bits),
            com_velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
            board_velocity: p.vectors_400_416[0].map(f32::from_bits),
            up: p.vectors_544_560_592_608[0].map(f32::from_bits),
            hips_up: hips[1],
            animation_right: p.effective_anim_transform_192[0].map(f32::from_bits),
            reversed: p.flags_2476 & 4 != 0,
        },
    );
    clear_ik(skater);
    let mut effects = skater.board_possession_live.effects(
        physics,
        &mut skater.ground_lifecycle.board_animated_290,
        p.timestep_2604,
    );
    effects.disable_animation();
    effects.standard_board();
    skater.board_possession_live.publish_volumes(physics);
    if skater.landing_on_deck.state.hippy {
        skater
            .skeleton_output
            .trigger_wobble(false, ((p.flags_2468 >> 20) ^ (p.flags_2476 >> 2)) & 1 != 0);
    }
    Ok(())
}
pub(crate) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Landing update requires the completed board toolkit")?;
    let board = toolkit.deck;
    let forward = toolkit.effective[2];
    let scene = super::offboard::contact_toolkit::StaticScene::new(&physics.world)
        .map_err(str::to_owned)?;
    let output = skater.landing_deck.update(&scene, &skater.player_input)?;
    skater.landing_on_deck.state.align(
        &skater.landing_on_deck.settings,
        forward,
        p.effective_anim_transform_192[2].map(f32::from_bits),
        p.state_timer_2664,
        f32::from_bits(p.vectors_544_560_592_608[3][1]),
        skater.animation_input.extra.physical_body_spin,
    );
    skeleton::update(physics, skater, output.position)?;
    skater.landing_on_deck.state.advance_spin(output);
    let manager = &skater.landing_deck.manager;
    if manager.trajectory_32.velocity_at(manager.elapsed_160)[1] < 0. {
        let time = State::accurate_time(
            board[3][1],
            f32::from_bits(p.vectors_400_416[0][1]),
            f32::from_bits(p.vectors_544_560_592_608[3][1]),
            [
                skater.skeleton.record.pose[15][3][1],
                skater.skeleton.record.pose[19][3][1],
            ],
            p.wheel_count_2556,
        );
        skater.landing_on_deck.state.time_to_land = time;
        skater.landing_on_deck.state.near_deck = time < 0.016000001;
        skater.landing_deck.calculate_accurate_ik_offset(
            &skater.player_input,
            &skater.animated_skeleton,
            skater.animated_skeleton.unadjusted_board[3],
            time,
        )?;
        if skater.landing_on_deck.state.near_deck {
            skater
                .skeleton_output
                .trigger_wobble(true, ((p.flags_2468 >> 20) ^ (p.flags_2476 >> 2)) & 1 != 0);
        }
    }
    if skater.animated_skeleton.unadjusted_board[2][2].abs() >= 0.97000003 {
        if p.flags_2484 & 2 == 0 && skater.landing_on_deck.state.time_to_land < 0.2 {
            skater.foot_ik.enable_feet(true);
        }
    } else {
        clear_ik(skater);
    }
    skater.landing_on_deck.state.finish(
        &skater.landing_on_deck.settings,
        f32::from_bits(p.vectors_544_560_592_608[3][1]),
    );
    Ok(())
}
///82D4DF08: synchronize actual query completion before rejection/collision tests.
pub(crate) fn post(skater: &mut Skate3ControllerState, frame: Frame) -> Result<(), String> {
    skater.landing_deck.post_physics(&skater.player_input)?;
    if !skater.landing_deck.fill().can_land_316 {
        skater.wipeout.state.request(24, 0.);
    }
    if skater.landing_on_deck.state.dangerous {
        skater.wipeout.state.request(27, 0.);
    }
    skater
        .wipeout
        .check_air_collision(&skater.player_input.processed, &frame)
}
///82D4D830 resets only the shared manager; outgoing state data is not another trajectory.
pub(crate) fn exit(_physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.landing_deck.reset();
    Ok(())
}
