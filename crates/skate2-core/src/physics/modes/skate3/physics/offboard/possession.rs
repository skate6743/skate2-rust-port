//! Native hand controller over the player's live pose and board observations.
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    physics::{contact_feedback::choose_surface, skeleton_animation_record::compose_affine},
    player::offboard::board_possession::{Processed, lifecycle::Observation},
};

pub(crate) struct Actions<'a> {
    pub state: &'a mut skate3_core::player::offboard::board_possession::State,
    pub settings: &'a skate3_core::player::offboard::board_possession::lifecycle::Settings,
    pub observation: &'a Observation,
    pub effects: super::board_effects::BoardEffects<'a>,
}
impl skate3_core::player::lifecycle::SkateboardControllerActions for Actions<'_> {
    fn hold_skateboard(
        &mut self,
        fields: &mut skate3_core::player::lifecycle::SkateboardControllerFields,
    ) {
        self.state.hold(fields, self.observation, &mut self.effects);
    }
    fn let_go_of_skateboard(
        &mut self,
        fields: &mut skate3_core::player::lifecycle::SkateboardControllerFields,
    ) {
        self.state
            .let_go(fields, self.observation, self.settings, &mut self.effects);
    }
}

pub(crate) fn observe(
    physics: &Skate3PhysicsState,
    skater: &Skate3ControllerState,
) -> Result<Observation, String> {
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Board controller requires this update's board toolkit")?;
    let roots = &skater.animated_skeleton.roots;
    let frames = &skater.skeleton_input.drive_frames;
    let physical_parts = skater.skeleton.part_transforms();
    Ok(Observation {
        processed: Processed {
            board_frame_64: toolkit.deck,
            player_frame_192: p
                .effective_anim_transform_192
                .map(|v| v.map(f32::from_bits)),
            position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
            velocity_912: p.vectors_880_896_912_928_944[2].map(f32::from_bits),
            direction_400: p.vectors_400_416[0].map(f32::from_bits),
            hide_direction_464: p.vectors_464_480_496_512_528[0].map(f32::from_bits),
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2488: p.flags_2488,
        },
        board_collision_flags_872: physics.riding.ground.collision_flags,
        //82C08818 produces840 by voting the four retained wheel surfaces.
        board_state_840: choose_surface(
            physics.riding.wheel_lines.physics_surfaces,
            std::array::from_fn(|i| physics.riding.ground.parts[i].in_contact),
            physics.riding.ground.collision_flags & 0x0200_0000 != 0,
        ),
        hand_contacts: [3, 7].map(|i| skater.collision_feedback.current[i]),
        physical_hand_positions: [3, 7].map(|i| physical_parts[i][3]),
        animation_board_frame_12624: frames[0],
        animation_hand_frames: [frames[3], frames[7]],
        attachment_frame_0: compose_affine(&roots.animation_to_world, &frames[0]),
    })
}

pub(crate) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if !skater.skateboard_controller.fields.system_on_452 {
        return Ok(());
    }
    let observation = observe(physics, skater)?;
    let offboard = &mut skater.offboard;
    let mut effects = super::board_effects::BoardEffects {
        board: &mut physics.board,
        animated: &mut skater.ground_lifecycle.board_animated_290,
        policy: &mut offboard.board_policy,
        standard_deck_drag: offboard.standard_deck_drag,
        timestep: skater.player_input.processed.timestep_2604,
    };
    offboard.possession.update(
        &mut skater.skateboard_controller.fields,
        &observation,
        &offboard.possession_settings,
        &mut effects,
    );
    Ok(())
}

pub(crate) fn publish(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let observation = observe(physics, skater)?;
    let bone = compose_affine(
        &skater.animated_skeleton.roots.animation_to_world,
        &skater.skeleton_input.drive_frames[11],
    );
    let value = skate3_core::player::offboard::board_possession::fill(
        &skater.skateboard_controller.fields,
        &skater.offboard.possession,
        &observation.processed,
        bone,
    );
    let output = &mut skater.player_input.physical.off_board;
    output.board_angle_36 = value.angle_36;
    output.board_angle_40 = value.angle_40;
    output.flag_311 = u8::from(value.held_311);
    output.free_board_312 = u8::from(value.free_312);
    output.returning_board_313 = u8::from(value.returning_313);
    output.hidden_board_321 = u8::from(value.hiding_321);
    output.dropping_board_322 = u8::from(value.flag_322);
    output.retrieving_board_323 = u8::from(value.flag_323);
    output.board_request_324 = u8::from(value.flag_324);
    Ok(())
}
