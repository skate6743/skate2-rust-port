//! Distinct State501 lifecycle: original S3 82D2E5C0/EF38/FFF8/30300/EF20.
mod input;
mod post;
mod publication;
mod settings;
mod update;
use super::{Skate3PhysicsState, Skate3ControllerState};
pub(crate) use post::post;
use skate3_core::player::offboard::{
    biped_air::recovered::{State, post::Settings},
    controller::PlacementInput,
};
use skate_data::collections::Collections;
pub(crate) use update::update;
pub(crate) struct BipedAir {
    pub state: State,
    checks: Settings,
}
impl BipedAir {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            state: State::default(),
            checks: settings::load(data)?,
        })
    }
}
///82DB3C78 at the early fixed-step phase, before current ProcessInput.
///82859E70 completes earlier trajectory work atA06C/A080 and invokes virtual
///slot16 atA0D0. Actual vtable82328688+16 points to82DB3C78.
pub(crate) fn consume_selector(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> Result<(), String> {
    let context = input::context(skater);
    //82C209E4 supplies the stock Air clearance parameter, not guessed geometry.
    let clearance = skater.offboard_air_selector.settings.deck_center_to_truck;
    skater
        .offboard_air_selector
        .consume(
            &physics.world,
            &physics.grind_world,
            &physics.world_proxies.solids,
            context,
            clearance,
        )
        .map(|_| ())
        .map_err(str::to_owned)
}
///82DB6E18 (PhysicalPlayerHiLOD slot 3, called by SkaterManager::UpdateAnimation1 82596AF0 ahead
///of the animation update): the selector's validity (+8497) and time (+8392) go to OffBoard +331
///and +120, and with a valid selection its trajectory's highest point and the time to it go to
///+240 and +156. The trajectory's launch velocity (+128) is read from the selector where the
///run-out attributes need it.
pub(crate) fn publish_selector(skater: &mut Skate3ControllerState) {
    let selector = &skater.offboard_air_selector.core;
    let off = &mut skater.player_input.physical.off_board;
    off.trajectory_valid_331 = u8::from(selector.just_changed_8497);
    off.trajectory_time_120 = selector.sampling.selection.scalar_8392;
    if selector.just_changed_8497 {
        let (apex, time) = selector.sampling.selection.trajectory_8144.highest_position();
        off.vector_240 = apex.map(f32::to_bits);
        off.scalar_156 = time;
        if skater.player_input.processed.state_2508 != 501 {
            crate::diagnostics::test_log::event(
                "skate3_offboard_prediction",
                format_args!(
                    "state={} time={} apex_time={time}",
                    skater.player_input.processed.state_2508, off.trajectory_time_120
                ),
            );
        }
    }
}
///Update5 82DB7298..72E8: a run-out requested outside the off-board states (Processed 2508 not
///500 or 501) fills a launch from the current state and starts the shared selector (82D6CA58).
///82DB3C78 completes it at the next tick's start, and a BipedAir entered in that tick takes it over.
pub(crate) fn launch_runout(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let packet = input::runout_launch_packet(skater)?;
    crate::diagnostics::test_log::event(
        "skate3_runout_launch",
        format_args!(
            "tick={} state={} velocity={:?} position={:?}",
            physics.ticks, skater.player_input.processed.state_2508, packet.velocity_0, packet.position_32
        ),
    );
    input::submit(physics, skater, packet)
}
pub(crate) fn enter(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.biped_air.state.reset();
    //State68 is the SAME manager consumed by state503; state64 is Biped.
    skater.landing_deck.reset();
    let entry = input::enter(skater);
    skater.biped_air.state.begin_enter_after_reset(entry);
    let mut feet = std::mem::take(&mut skater.offboard_feet);
    super::offboard::air_feet::enter(skater, &mut feet);
    skater.offboard_feet = feet;
    skater
        .ground_lifecycle
        .skeleton_controller
        .request(4, &mut skater.skeleton_collision)?;
    if !skater
        .offboard_air_selector
        .core
        .sampling
        .preinitialized_8494
    {
        let packet = input::launch_packet(skater)?;
        input::submit(physics, skater, packet)?;
    }
    let p = &skater.player_input.processed;
    //82D7B7A0 is the common Biped placement, not Ground Enter/Update.
    skater.biped_ground.controller.place(PlacementInput {
        frame: entry.animation_frame,
        velocity: p.vectors_544_560_592_608[3].map(f32::from_bits),
        body_position: entry.body_position_15872,
        current_state: p.state_2508,
        previous_state: p.state_2504,
        previous_frame: p
            .effective_anim_transform_192
            .map(|v| v.map(f32::from_bits)),
    });
    skater.biped_air.state.finish_enter(input::enter(skater));
    Ok(())
}
pub(crate) fn exit(skater: &mut Skate3ControllerState) {
    skater
        .biped_air
        .state
        .exit(&mut skater.offboard_air_selector.core.sampling);
}
pub(crate) fn fill(skater: &mut Skate3ControllerState) -> Result<(), String> {
    let physical = &mut skater.player_input.physical;
    publication::publish(skater.biped_air.state.output(), &mut physical.off_board);
    super::offboard::air_feet::publish(&skater.offboard_feet, &mut physical.off_board);
    //82D7948C/98: destination is PhysOut24 collision, NOT Skeleton20.
    skater
        .landing_deck
        .publish(super::offboard::landing_deck::PublishTargets {
            off_board: &mut physical.off_board,
            skeleton_position_48: &mut physical.collision.vector_48,
            skeleton_flag_3482: &mut physical.collision.flag_3482,
        });
    Ok(())
}
