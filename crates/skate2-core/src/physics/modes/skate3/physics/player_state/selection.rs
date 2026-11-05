//! CalcSuggestedState82D8ADE8 gets actual contacts, line distances and pose.
use super::*;
use skate3_core::player::selector::{
    ProcessedStateInput, StateSelectionInput,
    conditions::{BoardBodyState, SkeletonAnimationState},
};
pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    snapshot: skate3_core::player::input_phase::ProcessedPhysicsSnapshot,
) -> Result<(), String> {
    debug_assert_eq!(snapshot.tick, physics.ticks);
    let p = &snapshot.input;
    let input = StateSelectionInput {
        processed: ProcessedStateInput {
            grind_type_1248: p.grind.family_1248,
            grind_candidate_1488: p.grind.valid_1488,
            grind_investigation_flags_1516: p.grind.flags_1516,
            field_1776: p.external_physics_1616.flags as i32,
            flags_2468: p.flags_2468,
            flags_2472: p.flags_2472,
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
            flags_2488: p.flags_2488,
            category_2512: p.category_2512,
            wheel_contact_count_2556: p.wheel_count_2556 as i32,
            field_2572: skater.player_state.post.state_frames,
            state_timer_2664: p.state_timer_2664,
            field_2732: skater.animation_input.fields.slide,
            field_2744: skater.animation_input.extra.revert_direction,
            trajectory_collision_time_2772: p.air_scalar_2772,
        },
        skateboard_contact_count_869: physics.riding.ground.wheel_contact_count,
        board_body: BoardBodyState {
            field_856: physics.riding.wheel_lines.minimum_distance,
            field_7692: physics.riding.ground.time_without_wheel_contact,
        },
        skeleton: SkeletonAnimationState {
            mode_16420: skater.skeleton_input.force_mode,
            back_chain_lane_12656: skater.skeleton_input.drive_frames[0][2][1],
            threshold_800: skater.player_state.animated_board_threshold,
        },
        normal_off_ground: skater.player_state.normal_off_ground,
        skitching_off_ground: skater.player_state.skitching_off_ground,
    };
    let current = skater.player_state.current();
    let requested = skater.player_state.selector.calculate(current, &input);
    if crate::diagnostics::test_log::enabled() && skater.animation_input.output.flags & (1 << 28) != 0 {
        // KickTurn attribute active: the animated deck that IsSkateboardAnimated82BDC6C8 tests,
        // its yaw, and the reckoning heading that GroundAnimation keeps.
        let deck = skater.skeleton_input.drive_frames[0][2];
        let heading = physics.riding.reckoning_frames.heading;
        crate::diagnostics::test_log::event("skate3_animated_deck", format_args!(
            "tick={} state={current:?}->{requested:?} deck_zy={} threshold={} mode={} deck_yaw={:.2} heading_yaw={:.2}",
            physics.ticks, deck[1], input.skeleton.threshold_800, input.skeleton.mode_16420,
            deck[0].atan2(deck[2]).to_degrees(), heading[0].atan2(heading[2]).to_degrees()));
    }
    skater.player_state.requested_state = requested;
    if requested != current {
        //82DB5FF4 restores the actual counter also used by ground-history.
        skater.player_input.player.ground_history_frames_1304 = 100;
        super::transition::set(physics, skater, requested)?;
    }
    //SystemLogic82DB6000 clears the per-frame Wipeout request accumulator.
    skater.wipeout.state.clear_after_selection();
    Ok(())
}
