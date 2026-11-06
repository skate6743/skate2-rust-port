//! Update3_State82DB6050 prefix through grab-spline query synchronization.
//! Ground/Air/KnownAir vtable24 ->82D34DA8 ->82D2D860 predicts from the actual deck part
//! transform and its physical body's linear velocity, with a single VMADD.
use super::*;
use skate3_core::physics::board::BodyId;
pub(super) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let player = &mut skater.player_input.player;
    player.state_count_1312 = player.state_count_1312.wrapping_add(1);
    //KnownAir ctor82D35130 installs82327210; vslot24 at82327228 is
    //the same82D34DA8. Its target-following trajectory is not this deck sample.
    //Six grind vtables call82D2D860 directly;701 calls the82D34DA8 wrapper.
    if !skater.player_state.current().is_grind()
        && !matches!(
            skater.player_state.current(),
            PhysicalStateId::PhysicsGround
                | PhysicalStateId::PhysicsAir
                    | PhysicalStateId::PhysicsAirSecondary
                | PhysicalStateId::FootPlant | PhysicalStateId::Boneless | PhysicalStateId::HandPlant | PhysicalStateId::RevertGround
                | PhysicalStateId::KnownAir
                | PhysicalStateId::BipedAir
                | PhysicalStateId::BipedGround
                | PhysicalStateId::OffBoardPushing
                | PhysicalStateId::GroundAnimation
                | PhysicalStateId::SlideGround
                | PhysicalStateId::WipeoutGround
                | PhysicalStateId::Teleporting
                | PhysicalStateId::LandingOnDeck
                | PhysicalStateId::Nonspecific
                // Skitching vtable 0x82327398 slot 6 is the same 0x82D34DA8.
                | PhysicalStateId::Skitching
        )
    {
        return Err("PreState requires the selected state's actual PredictFutureOfDeck".into());
    }
    let deck = physics.board.part_transforms()[BodyId::Deck.index()];
    let velocity = physics.board.bodies()[BodyId::Deck.index()]
        .rates
        .linear_velocity;
    let dt = skater.player_input.processed.timestep_2604;
    let prediction = [
        velocity.x.mul_add(dt, deck.translation.x),
        velocity.y.mul_add(dt, deck.translation.y),
        velocity.z.mul_add(dt, deck.translation.z),
        0.0,
    ];
    let roots = &mut skater.animated_skeleton.roots;
    roots.predicted_board_position = prediction;
    roots.supplied_prediction = Some(prediction);
    //82DB60DC clears sticky3184, not this-update3185.
    skater.foot_ik.state.contacts.support_failed = false;
    //82DB60EC calls82D74270 AFTER sticky reset and before state PreUpdate.
    //Consume the canonical owner's preceding validation/query results.
    let p = &skater.player_input.processed;
    let context = skate3_core::player::offboard::ground_query::QueryContext {
        selection_flags_2948: p.actor_query_2948,
        matching_id_2952: p.actor_query_2952 as i32,
    };
    let scene = super::super::offboard::grab_scene::Scene::new(
        &physics.world,
        &physics.offboard_grab_scene,
    );
    skater.offboard_grab.sync(&scene, context)?;
    Ok(())
}
