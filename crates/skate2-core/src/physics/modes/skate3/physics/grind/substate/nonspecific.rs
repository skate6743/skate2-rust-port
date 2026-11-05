//!82D42DF0 through board helper82D42F50; caller owns final grind reckoning.
use super::{Family, Skate3PhysicsState, LaunchInput, Skate3ControllerState, collision_force, skeleton};
use skate3_core::{
    player::selector::conditions::{SkeletonAnimationState, is_skateboard_animated},
    riding::anti_flip::{self, AntiFlipInput},
};

pub(crate) fn execute(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.grind.nonspecific_jumped = false;
    let p = &skater.player_input.processed;
    if p.flags_2468 & 0x0040_0000 != 0 {
        let position = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Nonspecific pop requires current BoardToolkit")?
            .deck[3];
        let input = LaunchInput {
            velocity_400: p.vectors_400_416[0].map(f32::from_bits),
            position_112: position,
            current_point_1120: p.grind.point_1120.map(f32::from_bits),
            balance_2800: skater.animation_input.extra.grind_stability_nudge,
            geometry_side_jump: f32::from_bits(0x3f33_3333),
            vertical_jump: skater.grind.settings.substate.vertical(
                Family::FiftyFifty,
                p.state_variant_index_2528,
                skater.animation_input.extra.jump_strength,
            )?,
        };
        let velocity = super::super::launch::apply(
            &mut physics.board,
            &mut skater.player_input.grind.jumper,
            &mut skater.player_input.processed.flags_2476,
            input,
        );
        skater.grind.nonspecific_jump_velocity = velocity;
        skater.grind.nonspecific_jumped = true;
        skeleton::animated(physics, skater)?;
        //82D42E8C reapplies velocity AFTER animated board/skeleton update.
        skater
            .ground_runtime
            .set_animated_velocity(&mut physics.board, velocity);
    } else {
        let animated = is_skateboard_animated(SkeletonAnimationState {
            mode_16420: skater.skeleton_input.force_mode,
            back_chain_lane_12656: skater.skeleton_input.drive_frames[0][2][1],
            threshold_800: skater.grind.settings.substate.animated_board_threshold,
        });
        let target = if animated && p.flags_2472 & 0x0040_0000 != 0 {
            skeleton::animated_target(physics, skater)?
        } else {
            skeleton::ground_target(physics, skater)?
        };
        //82D42F50 starts with steering0 then82C04368, before angular/support.
        let p = &skater.player_input.processed;
        skater.ground.steering.update(
            0.,
            skater.ground_settings.board().steering.tilt_blending,
            p.flags_2468,
            p.flags_2472,
        );
        skater
            .skeleton_air
            .capture_physics_error(&physics.board, &target);
        let toolkit = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("Nonspecific board update requires current BoardToolkit")?;
        let up = p.vectors_544_560_592_608[0].map(f32::from_bits);
        let correction = anti_flip::calculate(
            skater.ground_settings.board().anti_flip,
            &AntiFlipInput {
                flags_2468: p.flags_2468,
                balance: skater.animation_input.fields.balance,
                axis_96: toolkit.deck[2],
                axis_64: toolkit.deck[0],
                projection_axis_544: up,
            },
        );
        //82C075B8 adds torque acceleration, not angular velocity/orientation.
        skater
            .ground_runtime
            .apply_angular_displacement(&mut physics.board, correction);
        super::super::board::apply_world_force(
            &mut physics.board,
            up.map(|v| v * -40.),
            toolkit.deck[3],
        );
        //Unlike common grind,701 discards optional velocity and torque outputs.
        collision_force(physics, skater)?;
    }
    Ok(())
}
