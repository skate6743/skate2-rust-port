//! PreUpdate82D30D30 inputs borrowed from their completed production owners.
use crate::skate3_native::physics::Skate3ControllerState;
use skate3_core::player::offboard::{ground_input::GroundInput, ground_job};

pub(crate) fn prepare(skater: &Skate3ControllerState) -> ground_job::Input {
    let p = &skater.player_input.processed;
    //ProcessAnimAttributes82BDA0D0 already owns these Processed fields in the
    //animation-input adapter. Read them directly rather than duplicating reset
    //and attribute dispatch in a second Processed record.
    let attributes = &skater.animation_input.fields;
    let extra = &skater.animation_input.extra;
    let frame = skater.offboard.ground.frame_80;
    //82D30DBC tests byte1092 of the third line record, then copies1056.
    let fallback = p.line_tests_960_1008_1056[2];
    ground_job::Input {
        previous_state: p.state_2504,
        frames_since_teleport: p.frames_since_teleport_2584,
        fallback_line_hit: (fallback.valid != 0).then(|| fallback.position.map(f32::from_bits)),
        controls: GroundInput {
            processed_flags_2472: p.flags_2472,
            processed_direct_2684: extra.offboard_magnitude,
            processed_direct_2680: extra.offboard_turn,
            processed_stick_2692: extra.biped_world_x,
            processed_stick_2688: extra.biped_world_z,
            processed_scale_2912: attributes.magnitude_scale,
            processed_scale_2908: attributes.turn_scale,
            frame_forward_112: [frame[2][0], frame[2][1], frame[2][2]],
        },
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        flags_2488: p.flags_2488,
        animation_motion_2864: attributes.animation_translation,
        duration_2896: attributes.animation_time,
        phase_2900: attributes.cadence_end_percent,
        override_duration_2904: attributes.animation_physics_blend_seconds,
        position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
        velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
        skeleton_motion_16320: skater.animated_skeleton.motion.velocity_world,
        skeleton_displacements_16288_16304: skater.collision_extra_displacements,
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires extracted private stock assets"]
    fn reset_attributes_and_third_line_fallback_reach_ground_job() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
        let assets = skate_data::GameAssets::load(&root).unwrap();
        let graphs = crate::skate3_native::graph_runtime::StockGraphs::load(&root, &assets).unwrap();
        let physics = crate::skate3_native::physics::Skate3PhysicsState::load_with_difficulty(
            &root,
            None,
            crate::skate3_native::difficulty::Difficulty::Hardcore,
        )
        .unwrap();
        let mut skater = Skate3ControllerState::load(&root, &graphs, &physics, "hardcore").unwrap();
        let lines = &mut skater.player_input.processed.line_tests_960_1008_1056;
        lines[0].valid = 1;
        lines[0].position = [99.0f32.to_bits(); 4];
        assert!(prepare(&skater).fallback_line_hit.is_none());
        skater.player_input.processed.line_tests_960_1008_1056[2].valid = 1;
        skater.player_input.processed.line_tests_960_1008_1056[2].position = [2.0f32.to_bits(); 4];
        skater.collision_extra_displacements = [[1.; 4], [2.; 4]];
        let input = prepare(&skater);
        //A reset cadence gate must be negative: zero activates the native
        //animation-velocity override and stops ordinary walking.
        assert_eq!(input.phase_2900, -1.);
        assert_eq!(input.controls.processed_scale_2912, 1.);
        assert_eq!(input.fallback_line_hit, Some([2.; 4]));
        assert_eq!(input.skeleton_displacements_16288_16304, [[1.; 4], [2.; 4]]);
    }
}
