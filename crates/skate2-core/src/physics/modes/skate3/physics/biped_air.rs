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
