//! Selected-state FillPhysOut and common state82DB7580..7790.
use super::*;
use skate3_core::{
    physics::{
        contact_feedback::choose_surface,
        filtered_state::{FilteredStateInput, GrindState},
    },
    player::input_phase::CurrentStateFields,
};
pub(super) fn publish(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let state = skater.player_state.current();
    super::super::offboard::board_manager::runtime::publish(physics, skater);
    //82DB7218..7258 runs for every state, before the selected state's Fill.
    //BipedAir may subsequently overwrite64 with its sampled trajectory velocity.
    let biped = &skater.biped_ground.controller.state;
    skater.player_input.physical.off_board.vector_64 =
        biped.frame_output.velocity.map(f32::to_bits);
    skater.player_input.physical.off_board.cadence_phase_80 = biped.cadence.phase.phase;
    skater.player_input.physical.off_board.locomotion_state_84 = biped.cadence.locomotion_index;
    if !state.is_grind()
        && state != PhysicalStateId::Nonspecific
        && !matches!(
            state,
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
        )
    {
        return Err(format!(
            "FillPhysOut requires the actual {state:?} output owner"
        ));
    }
    if state == PhysicalStateId::KnownAir {
        super::super::known_air::fill(physics, skater)?;
    }
    if state == PhysicalStateId::BipedAir {
        super::super::biped_air::fill(skater)?;
    }
    if state == PhysicalStateId::LandingOnDeck {
        super::super::landing_on_deck::fill(skater)?;
    }
    if state.is_grind() || state == PhysicalStateId::Nonspecific {
        super::super::grind::fill(physics, skater)?;
    }
    let offboard_ground = if matches!(
        state,
        PhysicalStateId::BipedGround | PhysicalStateId::OffBoardPushing
    ) {
        let output = super::super::biped_ground::fill(skater, &skater.offboard_feet)?;
        Some(super::super::biped_ground::publish_fields(
            output,
            &mut skater.player_input.physical,
        ))
    } else {
        None
    };
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("State output requires actual board toolkit")?;
    let output = (state == PhysicalStateId::PhysicsGround)
        .then(|| skater.ground.output(p, &skater.animation_input, toolkit));
    let player = &mut skater.player_input.player;
    skater.player_state.state_count = player.state_count_1312;
    skater.player_state.update_count = player.update_count_1316;
    let flags = &mut skater.player_state.state_flags;
    //State template82DE3BE8 clears bytes52..87. Only source-owned writes follow.
    *flags = [false; 36];
    let mut set = |offset: usize, value: bool| flags[offset - 52] = value;
    set(52, p.flags_2468 & (1 << 30) != 0);
    set(54, p.flags_2468 & (1 << 29) != 0);
    set(55, p.flags_2468 & (1 << 25) != 0);
    set(56, p.flags_2468 & (1 << 27) != 0);
    set(57, p.flags_2468 & (1 << 26) != 0);
    set(58, p.flags_2468 & (1 << 28) != 0);
    set(59, p.flags_2468 & (1 << 18) != 0);
    set(60, skater.animation_input.fields.balance != 0.0);
    set(62, p.flags_2472 & (1 << 10) != 0);
    set(
        63,
        state == PhysicalStateId::LandingOnDeck
            && skater.landing_on_deck.state.requests_board_flip(),
    );
    // State63 is set only by LandingOnDeckManager::Fill82D4E0E8..E0F4
    // when byte246 && time240<.02. Ordinary Ground never invokes that Fill,
    // so it retains the template zero here; State65 carries Ground requests.
    set(66, state == PhysicalStateId::RevertGround && skater.revert_state.active);
    set(65, skater.wipeout.requests_wipeout(p));
    set(78, skater.wipeout.requests_runout(p));
    if skater.wipeout.state.count != 0 {
        let c = &skater.collision_feedback;
        crate::skate3_native::physics::shove_probe::trace_wipeout(state, &skater.wipeout.state.reasons,
            skater.wipeout.requests_wipeout(p), skater.wipeout.requests_runout(p),
            [c.maximum_group_8_force, c.maximum_skater_force, c.maximum_group_11_force]);
    }
    set(71, player.flags_1296 & (1 << 24) != 0);
    set(73, p.flags_2476 & (1 << 24) != 0);
    set(74, state == PhysicalStateId::HandPlant && skater.handplant.continuation);
    set(75, state.category() == 500);
    set(76, state as u32 == 500);
    //82DB78A8..78DC: the processed off-board request refreshes three
    //outputs, then this publication consumes one and exposes State77.
    if p.state_identifier_2496 == 500 {
        player.dismount_request_frames_1332 = 3;
    }
    let dismount_requested = player.dismount_request_frames_1332 > 0;
    if dismount_requested {
        player.dismount_request_frames_1332 -= 1;
    }
    set(77, dismount_requested);
    set(79, p.state_variant_index_2528 == 3);
    set(80, p.flags_2484 & (1 << 21) != 0);
    set(83, skater.footplant.perform || skater.footplant.flag_627);
    if let Some(output) = &output {
        set(84, output.state_28.flag_84);
        set(86, output.state_28.has_world_grab_intent_without_object);
        if let Some(value) = output.state_28.manual_correction_write_78 {
            set(78, value);
        }
    }
    set(87, p.flags_2484 & (1 << 10) != 0);
    //Update582DB7B68..7C14: a pushing tick (2468 bit25) is mongo when the
    //right-toe bit equals AnimOut riding-switch == regular-stance. HiLOD1296
    //bits22/23 keep the previous tick, so each stroke publishes once.
    let packet = &skater.animation.packet;
    let (mongo, regular) = if p.flags_2468 & (1 << 25) != 0 {
        let mongo = (p.flags_2468 & (1 << 26) != 0) == (packet.riding_switch == packet.regular_stance);
        (mongo, !mongo)
    } else {
        (false, false)
    };
    let first_mongo = mongo && player.flags_1296 & (1 << 22) == 0;
    let first_regular = !first_mongo && regular && player.flags_1296 & (1 << 23) == 0;
    skater.player_state.push_style = [first_mongo, first_regular];
    player.flags_1296 = (player.flags_1296 & !0x00c0_0000) | (u32::from(mongo) << 22) | (u32::from(regular) << 23);
    if let Some(output) = &offboard_ground {
        set(86, output.flag_86);
    }
    if state == PhysicalStateId::SlideGround {
        set(84, skater.slide_state.state.wall_riding);
    }
    let physical = &mut skater.player_input.physical;
    physical.component_1832_word_1876=skater.handplant.flags;
    physical.air.handplant_position_304=skater.handplant.anchor.map(f32::to_bits);
    physical.air.handplant_time_320=skater.handplant.phase;
    physical.air.flag_446=u8::from(skater.handplant.flags&0x8000_0000!=0);
    skater.footplant.publish(&mut physical.air);
    //82DB76E0..76F4 publishes this independently of selected-state304/311.
    physical.off_board.flag_308 = u8::from(p.flags_2480 & (1 << 19) != 0);
    //Common ProcessOutput82DB7044/7048 and70C0/70C4; these precede state Fill.
    physical.air.flag_444 = u8::from(
        skater
            .trajectory
            .selector
            .selection()
            .is_some_and(|s| s.wall_ride),
    );
    physical.air.flag_441 = u8::from(skater.air_reckoning.state.flip_active);
    physical.air.flag_445 = u8::from(skater.air_reckoning.state.flip_side);
    if state == PhysicalStateId::GroundAnimation {
        skater.ground_animation.fill(p, &mut physical.air);
    }
    physical.state = CurrentStateFields {
        category_12: state.category(),
        state_16: state as u32,
        flag_66: u8::from(state == PhysicalStateId::RevertGround && skater.revert_state.active),
        flag_74: u8::from(state==PhysicalStateId::HandPlant && skater.handplant.continuation),
        //ProcessOutput82DB7104..7128: selector57 requests State69 here.
        //The next input/selection enters702; never reset inside selection.
        flag_69: u8::from(skater.player_state.selector.request_teleport),
        ..CurrentStateFields::default()
    };
    skater.player_state.state_flags[69 - 52] = physical.state.flag_69 != 0;
    if let Some(output) = &offboard_ground {
        physical.state.counter_36 = output.word_36;
        physical.state.skitch_value_40 = output.word_40;
    }
    if let Some(output) = &output {
        physical.state.skitch_value_40 = output.state_28.grab_spline_object_id;
        physical.state.signed_ground_step_84 = u8::from(output.state_28.flag_84);
        physical.ground.vector_128 = output.ground_32.anti_flip_torque.map(f32::to_bits);
        physical.ground.scalar_276 = output.ground_32.time_to_skitch;
        physical.ground.flag_317 = u8::from(output.ground_32.anti_flip_nudge_present);
        physical.ground.flag_318 = u8::from(output.ground_32.is_pinning);
        physical.off_board.flag_304 = u8::from(output.is_grabbing_object_72_304);
        physical.animation.manual_opposition_168 = u8::from(output.manual_opposition_56_168);
    } else if state == PhysicalStateId::PhysicsAir {
        let air = skate3_core::air::state::fill_physics_output(&skater.air_state);
        physical.air.landing_normal_144 = air.landing_normal.map(f32::to_bits);
        physical.air.jump_height_200 = air.jump_height;
        physical.air.reached_apex_436 = u8::from(air.is_at_apex);
        if let Some(value) = air.scalar_184_write {
            physical.air.scalar_184 = value;
        }
    } else if state == PhysicalStateId::SlideGround {
        //State Fill82D3B010 is a single byte from this retained Slide owner.
        physical.state.signed_ground_step_84 = u8::from(skater.slide_state.state.wall_riding);
    }
    //ProcessOutput82DB71C4 calls SkateboardController::FillPhysOut82D76D20
    //for every selected state. The constructor82D74DD8 seeds448=0; use the
    //same controller owner as state transitions and post-physics ragdoll.
    //Published by board_manager::runtime::publish above from retained controller state.

    //ProcessOutput82DB7130..7180 copies the HandPlantManager animation
    //flags1876 into Air304+20 one bit at a time, preserving the low28 bits.
    //The same manager word is consumed by the actual input publication.
    physical.air.handplant_flags_324 = (physical.air.handplant_flags_324 & 0x0fff_ffff)
        | (physical.component_1832_word_1876 & 0xf000_0000);

    if let Some(projection) = output
        .as_ref()
        .and_then(|output| output.velocity_projection_36)
    {
        physical.reckoning.vector_144 =
            projection.velocity_without_axis_component.map(f32::to_bits);
        physical.reckoning.flag_164 = u8::from(projection.active);
    }
    let surface = choose_surface(
        physics.riding.wheel_lines.physics_surfaces,
        core::array::from_fn(|i| physics.riding.ground.parts[i].in_contact),
        physics.riding.ground.collision_flags & (1 << 25) != 0,
    );
    super::super::grind::condition(physics, skater)?;
    let physical = &mut skater.player_input.physical;
    let filtered = skater.player_state.filtered.update(FilteredStateInput {
        physics_category: state.category() as i32,
        physics_state: state as i32,
        anything_in_contact: physical.collision.flag_3477 != 0,
        physics_surface_type: surface as i32,
        wall_ride_exit: output
            .as_ref()
            .is_some_and(|output| output.ground_32.wall_ride_exit),
        //Original reset records; the selected ordinary Ground state and
        //authored empty-edge world do not publish the active air/grind fields.
        targeting_grind: state == PhysicalStateId::KnownAir
            && skater.known_air.state.targeting_grind_213,
        //82DE6078..60C0: state501 is Offboard or OffboardAir from output320.
        offboard_has_landed: physical.off_board.flag_320 != 0,
        offboard_on_deck: physical.off_board.flag_315 != 0,
        grind: if state.is_grind() {
            super::super::grind::Runtime::filtered_output(&physical.grinds)?
        } else {
            GrindState::default()
        },
        last_grind_distance: skater.grind.last_grind_distance(),
    });
    physical.collision.surface_type_16 = surface;
    physical.air.surface_category_232 = skater.trajectory.selector.selection()
        .map_or(0, |selection| selection.surface_category);
    physical.filtered_state_0 = filtered.category as u32;
    skater.player_state.filtered_output = Some(filtered);
    skater.player_state.ground_output = output;
    if state == PhysicalStateId::WipeoutGround {
        super::wipeout_output::publish(skater);
    }
    if state == PhysicalStateId::Teleporting {
        skater
            .teleport_state
            .publish_output(&mut skater.player_input.physical);
    }
    // ProcessOutput calls 82DB8120 after component and selected-state Fill.
    skate3_core::player::input_phase::publish_special_surface(
        &mut skater.player_input.physical,
        &skater.player_input.processed,
        &mut skater.player_state.state_flags,
        physics.riding.ground.collision_flags,
        physics.riding.ground.surface_twelve_height,
    );
    // A manual target spans the initial reset and state702's continuation.
    // Release it only after the real source state has completed that reset.
    skater.teleport_state.finish_manual_reset(state);
    Ok(())
}
