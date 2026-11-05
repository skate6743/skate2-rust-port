//! State501 Post82D2FFF8, after completed physical skeleton feedback.
use super::Skate3ControllerState;
use skate3_core::player::{offboard::biped_air::recovered::post::PostInput, wipeout::Frame};
pub(crate) fn post(skater: &mut Skate3ControllerState, frame: Frame) -> Result<(), String> {
    //82D30018..30: synchronize the SAME manager state503 will consume.
    skater.landing_deck.post_physics(&skater.player_input)?;
    let p = &skater.player_input.processed;
    let mode = skater.wipeout.mode(p)?;
    skater.biped_air.state.post_physics(
        PostInput {
            flags_2484: p.flags_2484,
            state_timer_2664: p.state_timer_2664,
            forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
            side_192: p.effective_anim_transform_192[0].map(f32::from_bits),
            input_2708: skater.animation_input.extra.look_y,
            input_2704: skater.animation_input.extra.look_x,
        },
        &skater.biped_air.checks,
        &frame,
        mode,
        skater.skeleton_input.root_velocity,
        &mut skater.wipeout.state,
    );
    Ok(())
}
