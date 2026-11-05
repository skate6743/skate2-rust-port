//! Direct processed/physical inputs for the distinct BipedAir lifecycle.
use super::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    physics::skeleton_animation_record::compose_affine,
    player::offboard::{
        air_launch::{DepartureGeometry, Packet, Processed},
        air_selector::Context,
        biped_air::recovered::{EnterInput, Frame, height::HeightInput},
    },
};
pub(super) fn context(skater: &Skate3ControllerState) -> Context {
    let p = &skater.player_input.processed;
    Context {
        selection_flags_2948: p.actor_query_2948,
        matching_group_2952: p.actor_query_2952 as i32,
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
    }
}
pub(super) fn effective_frame(skater: &Skate3ControllerState) -> Frame {
    //82BE3650: actual animation root, with effective stance axes0/2.
    let mut frame = skater.animated_skeleton.roots.animation_to_world;
    if skater.player_input.processed.flags_2476 & 4 != 0 {
        for axis in [0, 2] {
            frame[axis] = frame[axis].map(|v| -v);
        }
    }
    frame
}
pub(super) fn enter(skater: &Skate3ControllerState) -> EnterInput {
    let p = &skater.player_input.processed;
    EnterInput {
        animation_frame: effective_frame(skater),
        body_position_15872: skater.animated_skeleton.board_frames.com_frame[3],
        body_position_15936: skater.animated_skeleton.board_frames.lifted_com_frame[3],
        position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        flags_2484: p.flags_2484,
    }
}
pub(super) fn height(skater: &Skate3ControllerState) -> HeightInput {
    let p = &skater.player_input.processed;
    let root = skater.animated_skeleton.roots.animation_to_world;
    HeightInput {
        bone15: compose_affine(&root, &skater.animated_skeleton.record.pose[15])[3],
        bone19: compose_affine(&root, &skater.animated_skeleton.record.pose[19])[3],
        //82BE3170 reads mapped physical target12624; NOT record6480.
        bone1: compose_affine(&root, &skater.skeleton_input.drive_frames[1])[3],
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
    }
}
pub(super) fn launch_packet(skater: &Skate3ControllerState) -> Result<Packet, String> {
    let p = &skater.player_input.processed;
    let board = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("BipedAir launch requires the completed BoardToolkit")?;
    let input = Processed {
        board_position_112: board.deck[3],
        forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
        velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
        velocity_912: p.vectors_880_896_912_928_944[2].map(f32::from_bits),
        departure_geometry: Some(DepartureGeometry {
            point_1120: p.grind.point_1120.map(f32::from_bits),
            axis_1136: p.grind.direction_1136.map(f32::from_bits),
        }),
        flags_2472: p.flags_2472,
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        previous_state_2504: p.state_2504,
        current_state_2508: p.state_2508,
        current_category_2512: p.category_2512,
        previous_category_2516: p.category_2516,
        raw_x_2692: skater.animation_input.extra.biped_world_x,
        raw_z_2688: skater.animation_input.extra.biped_world_z,
    };
    let (turn, settings) = skater.biped_ground.air_settings();
    skater
        .biped_air
        .state
        .launch_packet(
            &input,
            &skater.biped_ground.controller.state,
            &turn,
            settings,
        )
        .map_err(str::to_owned)
}
pub(super) fn submit(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    packet: Packet,
) -> Result<(), String> {
    let context = context(skater);
    let g = physics.settings.step.simulation.gravity_acceleration;
    skater
        .offboard_air_selector
        .launch(&physics.world, packet, [g.x, g.y, g.z, 0.], context)
        .map_err(str::to_owned)
}
