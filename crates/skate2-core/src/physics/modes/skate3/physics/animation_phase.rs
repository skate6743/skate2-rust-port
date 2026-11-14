//! The production physical-output -> stock graph -> next physical-input phase.
//! Native actor phases82592FD8/82593128/82593230/82593640 preserve this order.
#[path = "animation_grind.rs"]
mod grind;
#[path = "animation_phase_packet.rs"]
mod packet;
use super::{Skate3PhysicsState, PlayerControls, skater::Skate3ControllerState};
use crate::skate3_native::{graph_runtime::StockGraphs, skater_animation::AnimationPhysical};
pub(crate) use packet::{AnimationPhaseOutput, AnimationProfile};
use skate3_core::{
    animation::{
        body_tilt, ground_acceleration,
        physical_feedback::{ControlFeedback, PhysicalFeedback, ReckoningFeedback},
        riding_fakie,
    },
    graph::conditions::{ConditionInputs, PushBrakeInputs},
    math::Vector3,
    physics::board::BodyId,
    riding::push_behaviors::PushFootFrame,
};

/// Observations are from the preceding completed physics tick. The current
/// controller map then advances both stock graphs; returned storage remains
/// alive while PlayerInput consumes packet(), hierarchy and ordered attributes.
pub(crate) fn advance(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &PlayerControls,
    graphs: &StockGraphs,
    profile: &AnimationProfile,
) -> Result<AnimationPhaseOutput, String> {
    skater.animation.motion.score_packet = Default::default();
    let p = &skater.player_input.processed;
    let manual_board_target = skater.teleport_state.manual_on_board();
    let graph_offboard = manual_board_target.map_or(skater.player_input.physical.state.category_12 == 500, |on_board| !on_board);
    let physical = &skater.player_input.physical;
    skater.animation.motion.manual_exit = Some(physical.animation.manual_opposition_168 != 0);
    skater.animation.motion.deck_yaw_pitch = Some([
        physical.skeleton.deck_yaw_536,
        physical.skeleton.deck_pitch_540,
    ]);
    // ProcessOutput82DB7250/7258 publishes shared Biped720/716 as
    //OffBoard80/84. BipedCadence/MatchCadence/LocoState consume that completed
    //physical packet, preserving the publication boundary across transitions.
    // The native publication is unconditional, including on-board frames
    //when a dismount node can capture MatchCadence before state500 is entered.
    skater.animation.motion.offboard_cadence_phase = Some(physical.off_board.cadence_phase_80);
    skater.animation.motion.offboard_locomotion_state =
        Some(physical.off_board.locomotion_state_84);
    // GroundSlopeType82BA7E80 and IsBipedGroundThin82BA8110 read the
    // completed OffBoard88/330 directly, without a state/category gate.
    skater.animation.motion.ground_slope_type = Some(physical.off_board.kind_88);
    skater.animation.motion.biped_ground_thin = Some(physical.off_board.flag_330 != 0);
    skater.animation.action.dropping_in = Some(physical.grinds.dropping_in_324 != 0);
    let deck = physics.board.part_transforms()[BodyId::Deck.index()];
    let (fakie, mirrored) = skater.animation.stance();
    // ToggleBoard consumes the completed FillPhysOut publication from the
    // preceding tick. Keep this input explicit so unsupported behavior cannot
    // silently run against fabricated board state.
    skater.animation.motion.toggle_board_physical =
        Some(crate::skate3_native::graph_host::motion_toggle_board::Physical {
            grabbing_object: physical.off_board.flag_304 != 0,
            holding_board: physical.off_board.flag_311 != 0,
            free_board: physical.off_board.free_board_312 != 0,
            retrieval_blocked: skater.player_state.state_flags[87 - 52],
            //ToggleBoard82BA968C consumes returning313, not retrieval323.
            retrieval_active: physical.off_board.returning_board_313 != 0,
            yaw_radians: physical.off_board.angle_36,
            pitch_radians: physical.off_board.angle_40,
        });
    skater.animation.motion.runout_physical = Some(crate::skate3_native::graph_host::motion_runout::Observation {
        offboard_flag_331: physical.off_board.trajectory_valid_331 != 0,
        offboard_velocity_128: physical.off_board.vector_64.map(f32::from_bits),
        reckoning_velocity_16: physical.reckoning.vector_16.map(f32::from_bits),
        reckoning_up_96: physical.reckoning.vector_96.map(f32::from_bits),
        skeleton_vector_0: skater.skeleton_input.drive_frames[0][0],
        animation_mirrored: mirrored,
    });
    let feedback = skater.physical_feedback;
    skater.animation.motion.wipeout_physical = Some(crate::skate3_native::graph_host::motion_wipeout::Physical {
        over_599: physical.skeleton.over_599 != 0,
        collision_time_144: physical.animation.collision_time_144,
        no_support_time_548: physical.skeleton.no_support_time_548,
        profile_148: physical.animation.profile_148,
        below_surface_82: skater.player_state.state_flags[82 - 52],
        // Skeleton Fill82BE1E94/98 copies hips physical up into output80.
        orientation_y: Some(skater.skeleton.record.pose[23][1][1]),
        hips_right_angle_496: physical.skeleton.hips_right_angle_496,
        hips_up_angle_500: physical.skeleton.hips_up_angle_500,
    });
    skater.animation.motion.landing_physical = Some(crate::skate3_native::graph_host::motion_landing::Physical {
        height: feedback.crouching.animation_height_72,
        spin: skater.landing_quality.spin_92,
        kind: skater.landing_quality.landing_type_96,
        last_good_landing_velocity: skater.animation.motion.riding.last_good_landing_velocity,
    });
    skater.animation.motion.prelanding_physical =
        Some(crate::skate3_native::graph_host::motion_spin::PrelandingPhysical {
            air_444: physical.air.flag_444 != 0,
            air_normal_144_y: f32::from_bits(physical.air.landing_normal_144[1]),
            animation_16_x: skater.animation_feedback.published_previous_lateral_tilt[0],
            com_velocity_y: f32::from_bits(physical.reckoning.vector_16[1]),
            offboard_316: physical.off_board.flag_316 != 0,
            offboard_319: physical.off_board.flag_319 != 0,
            offboard_time_32: physical.off_board.scalar_32,
            air_437: physical.air.known_air_valid_437 != 0,
            air_normal_36: f32::from_bits(physical.air.landing_normal_32[1]),
            air_remaining_184: physical.air.scalar_184,
            animation_height_72: feedback.crouching.animation_height_72,
        });
    let [right_toe, left_toe] = skater
        .foot_ik
        .physical_toe_positions(&skater.skeleton.record);
    skater.animation.motion.air_leg_physical =
        Some(skate3_core::animation::air_leg_extension::Physical {
            com_velocity: physical.reckoning.vector_16.map(f32::from_bits),
            com_position: physical.reckoning.vector_64.map(f32::from_bits),
            system_up: physical.reckoning.vector_96.map(f32::from_bits),
            right_toe,
            left_toe,
            animation_height: feedback.crouching.animation_height_72,
            offboard_316: physical.off_board.flag_316 != 0,
            remaining_air_time: physical.air.scalar_184,
        });
    skater.animation.action.physical_conditions =
        Some(crate::skate3_native::graph_host::action::PhysicalConditions {
            requests_dismount: skater.player_state.state_flags[77 - 52],
            state: physical.state.state_16,
        });
    skater.animation.motion.gameplay_conditions = Some(
        crate::skate3_native::graph_host::motion_gameplay_conditions::GameplayConditions {
            state: physical.state.state_16,
            wants_runout: skater.player_state.state_flags[78 - 52],
            physics_wiping: skater.player_state.state_flags[59 - 52],
            body_flipping: physical.air.flag_441 != 0,
            bumped: feedback.bumped,
            wants_wipeout: skater.player_state.state_flags[63 - 52]
                || skater.player_state.state_flags[65 - 52],
            grabbing_object: physical.off_board.flag_304 != 0,
            retrieving_board: physical.off_board.retrieving_board_323 != 0,
            dropping_board: physical.off_board.dropping_board_322 != 0,
            in_biped_air: physical.off_board.flag_328 != 0,
            hippy_hurdling: physical.off_board.hippy_hurdling_317 != 0,
            handplant_flags: physical.air.handplant_flags_324,
            handplant_time: physical.air.handplant_time_320,
            handplant_thresholds: skater.handplant.animation_thresholds(),
            footplant_active: physical.air.flag_448 != 0,
            footplant_duration: physical.air.footplant_duration_212,
            footplant_contact_time: physical.air.footplant_contact_time_208,
            time_to_skitch: physical.ground.scalar_276,
            skitch_transition_time: profile.skitch_transition_time,
            time_to_land: physical.air.scalar_184,
            time_to_land_valid: physical.air.known_air_valid_437 != 0,
            offboard_trajectory_time: physical.off_board.trajectory_time_120,
            offboard_trajectory_valid: physical.off_board.trajectory_valid_331 != 0,
            trucks_or_deck_contact: physical.collision.flag_3472 != 0 || physical.collision.flag_3475 != 0,
            offboard_time_to_land: physical.off_board.scalar_32,
            offboard_air_scalar_92: physical.off_board.scalar_92,
            offboard_air_translation: physical.off_board.vector_96.map(f32::from_bits),
            offboard_landing_normal: physical.off_board.vector_192.map(f32::from_bits),
            offboard_committed_to_motion: physical.off_board.flag_329 != 0,
            offboard_obstacle_distance: physical.off_board.scalar_112,
            offboard_edge_distance: physical.off_board.distance_116,
            //ApexReached82BA5C78 reads Reckoning16.Y, not Air436.
            reached_apex: f32::from_bits(physical.reckoning.vector_16[1]) < 0.0,
            can_land_on_board: physical.off_board.flag_316 != 0,
            landing_turning: physical.off_board.flag_318 != 0,
            grind_contact: physical.grinds.flag_318 != 0 || physical.grinds.flag_322 != 0,
            wheel_contact: physical
                .collision
                .wheel_contact_3296_3299
                .iter()
                .any(|&v| v != 0),
            tricks_blocked_on_stairs: false,
            moving_object: physical.state.state_16 == 502,
        },
    );
    skater.animation.action.gameplay_conditions = skater.animation.motion.gameplay_conditions;
    // Skitching FillPhysOut's Animation/Ground values, or Ground's own spline height (Ground+280).
    let skitch = skater.skitch_output.unwrap_or_default();
    skater.animation.motion.skitch_physical =
        Some(crate::skate3_native::graph_host::motion_skitching::Physical {
            state: physical.state.state_16,
            crouch: skitch.crouch,
            hand_speed: skitch.hand_speed,
            grip_hand: skitch.grip_hand,
            time_to_skitch: physical.ground.scalar_276,
            spline_height: if skater.skitch_output.is_some() {
                skitch.spline_height
            } else {
                skater
                    .player_state
                    .ground_output
                    .as_ref()
                    .map_or(0.0, |output| output.ground_32.skitch_spline_height)
            },
            contact_speed: skitch.contact_speed,
            push_speed: skitch.push_target_speed,
        });
    skater.animation.motion.native_physical = Some(crate::skate3_native::graph_host::motion_native::Physical {
        centre_of_mass_velocity: skater.animated_skeleton.board_frames.com_velocity,
        system_up: vec4(physics.riding.reckoning.up),
        //82DB70A0..70FC publishes Reckoning752, not the physical deck frame.
        board_reckoning_z: physics.riding.reckoning_frames.ground[2],
        board_reckoning: physics.riding.reckoning_frames.ground,
    });
    skater.animation.motion.bump_acceleration = Some(feedback.ground_acceleration);
    skater.animation.motion.gesture_physical =
        Some(crate::skate3_native::graph_host::motion_native::GesturePhysical {
            //Original82DB74D8, not the similarly numbered Air output byte.
            ground321: p.flags_2480 & 0x1000 != 0,
            state_offboard75: graph_offboard,
            selections: profile.gesture_selections,
            suppress_up: profile.suppress_up_gesture,
            force_brake_bypass: profile.gesture_force_brake_bypass,
        });
    //Original ProcessOutput82DB7E38. Probe+72 is copied to Processed1848;
    //Ground240 resets to zero and receives this point only on an interaction.
    let interaction_trigger = matches!(p.category_2512, 100 | 500)
        && p.probe_1792.bytes_72_73[0] != 0
        && p.flags_2476 & 0x400000 != 0;
    let direction = if interaction_trigger {
        let source = if p.flags_2484 & 0x40000 != 0 { 0 } else { 1 };
        let point = p.probe_1792.vectors_16_32_48[source].map(f32::from_bits);
        let inverse = skater.animated_skeleton.roots.world_to_animation;
        std::array::from_fn(|i| {
            inverse[2][i].mul_add(
                point[2],
                inverse[1][i].mul_add(point[1], inverse[0][i].mul_add(point[0], inverse[3][i])),
            )
        })
    } else {
        [0.0; 4]
    };
    // The hit branch: a strike (Processed1849 with flags 2484 bit 19) moves Ground240 to the
    // target and hands the game a type 2 (pedestrian) hit with 0x82db7fc0's velocity.
    let shove_hit = matches!(p.category_2512, 100 | 500)
        && p.probe_1792.bytes_72_73[1] != 0
        && p.flags_2484 & 0x80000 != 0;
    if p.probe_1792.bytes_72_73[0] != 0 {
        super::shove_probe::trace(format_args!(
            "output: category {} found {} hit {} 2476 bit22 {} 2484 bit19 {} -> trigger {interaction_trigger} hit {shove_hit}",
            p.category_2512, p.probe_1792.bytes_72_73[0], p.probe_1792.bytes_72_73[1],
            p.flags_2476 & 0x400000 != 0, p.flags_2484 & 0x80000 != 0));
    }
    let direction = if shove_hit {
        let point = p.probe_1792.vectors_16_32_48[0].map(f32::from_bits);
        let inverse = skater.animated_skeleton.roots.world_to_animation;
        std::array::from_fn(|i| {
            inverse[2][i].mul_add(
                point[2],
                inverse[1][i].mul_add(point[1], inverse[0][i].mul_add(point[0], inverse[3][i])),
            )
        })
    } else {
        direction
    };
    if shove_hit {
        super::shove_probe::trace(format_args!("hit inputs: target {:?} target velocity {:?} system {:?} velocity {:?}",
            p.probe_1792.vectors_16_32_48[0].map(f32::from_bits), p.probe_1792.vectors_16_32_48[2].map(f32::from_bits),
            p.vectors_544_560_592_608[2].map(f32::from_bits), p.vectors_400_416[0].map(f32::from_bits)));
    }
    skater.shove.hit = (shove_hit && matches!(p.probe_1792.words_64_68[0], 1 | 2)).then(|| super::shove_probe::ShoveHit {
        id: p.probe_1792.words_64_68[1],
        skater: p.probe_1792.words_64_68[0] == 1,
        velocity: super::shove_probe::shove_velocity(
            p.probe_1792.vectors_16_32_48[0].map(f32::from_bits),
            p.probe_1792.vectors_16_32_48[2].map(f32::from_bits),
            p.vectors_544_560_592_608[2].map(f32::from_bits),
            p.vectors_400_416[0].map(f32::from_bits),
        ),
    });
    skater.animation.motion.shove_physical = Some(crate::skate3_native::graph_host::motion_shove::ShovePhysical {
        interaction_trigger,
        direction,
        in_biped_category: physical.state.category_12 == 500,
        board_on_ground: physical.off_board.flag_311 != 0,
        animation_height: feedback.crouching.animation_height_72,
    });
    let mut physical_state = grind::publish(
        &mut skater.animation.motion,
        physical,
        skater.player_state.filtered_output.as_ref(),
        feedback.crouching.animation_height_72,
        mirrored,
        physics.riding.motion.effective_basis.columns[2],
    )?;
    // TeleportPlayerNow runs in Update1, before the native graph reads that frame's filtered
    // Teleport. Here the graph dispatches before the physics input phase, and the State61
    // placement (whose skeleton reset82BE3508 clears Skeleton16420) runs while the state is
    // still 702, after the request itself is consumed. Until then the graph reads Teleport, so
    // OnGround's ForcePhysics Begin follows that reset as it does natively.
    let resetting = skater.player_input.pending_teleport().is_some()
        || skater.teleport_state.has_pending_reply()
        || skater.player_state.current() == skate3_core::player::state::PhysicalStateId::Teleporting;
    // The host marker target survives the ordinary702 publication.
    // Graph observations follow the reset continuation before its board target.
    if manual_board_target.is_some() {
        // Both targets enter Motion.Teleport for its animation/stance reset,
        // then observe the Ground continuation used by enter_after_teleport.
        // Forcing Offboard here skips the ActionGraph's Ground exit from its
        // Teleport leaf: its precondition is no longer evaluated while active,
        // so it never forwards walking intents. State77 and the authored
        // TeleportOffBoard transition still own the subsequent dismount.
        physical_state.category = if resetting { 5 } else { 1 };
    } else if resetting {
        physical_state.category = 5;
    }
    let conditions = ConditionInputs {
        speeds: Some(physics.riding.graph_speeds()),
        physical_state: Some(physical_state),
        time_since_last_input: Some(p.time_since_last_input_2748),
        mirrored: Some(mirrored),
        riding_fakie: Some(fakie),
        push_brake: Some(PushBrakeInputs {
            // Board FillPhysOut82C03304/3318: CollisionInfo+16 ->Ground80.
            ground_axis_y: physics.riding.ground.wheel_normal.y,
            skeleton_disables_push_brake: skater.foot_ik.state.contacts.support_failed_this_update,
            maximum_ground_angle_degrees: profile.maximum_ground_angle_degrees,
        }),
    };
    // Skeleton GetEffectiveRoot82BE3650 negates X/Z iff Processed2476 bit2.
    let mut effective_z = skater.animated_skeleton.roots.animation_to_world[2];
    if p.flags_2476 & 4 != 0 {
        effective_z = effective_z.map(|v| -v);
    }
    let mut effective_x = skater.animated_skeleton.roots.animation_to_world[0];
    if p.flags_2476 & 4 != 0 {
        effective_x = effective_x.map(|v| -v);
    }
    skater.animation.motion.riding_conditions = Some(
        crate::skate3_native::graph_host::motion_riding_conditions::RidingConditionInputs {
            com_velocity: physical.reckoning.vector_16.map(f32::from_bits),
            skeleton_x: effective_x,
            skeleton_z: effective_z,
            skate_up_y: deck.basis.columns[1][1],
            surface_up_y: physics.riding.ground.wheel_normal.y,
        },
    );
    let observations = AnimationPhysical {
        conditions,
        feedback,
        body_tilt: body_tilt::Physical {
            lateral_tilt: physics.riding.reckoning_frames.lateral_tilt[0],
            body_spin_speed: skater.air_reckoning.state.spin_speed,
            filtered_category: physical.filtered_state_0,
        },
        fakie: riding_fakie::Physical {
            category: physical.filtered_state_0,
            grind_state: physical.state.state_16,
            doing_trick: skater.animation.motion.flags.doing_trick,
            board_axis: effective_z,
            deck_velocity: vec4(physics.riding.motion.linear_velocity),
            external_velocity: skater.centre_of_mass_output.velocity,
            ground_projected_speed: physics.riding.motion.ground_speed,
        },
        // Original82DB7A58..7AA4 copies AnimOut10374/10371 respectively.
        physical_stance: (
            skater.animation.packet.regular_stance,
            skater.animation.packet.riding_switch,
        ),
        foot_frame: PushFootFrame {
            left_foot: skater.skeleton.record.pose[15][3],
            right_foot: skater.skeleton.record.pose[19][3],
            //82C02D0C: physical deck body+16, not geometric part translation.
            deck_position: vec4(physics.board.bodies()[BodyId::Deck.index()].rates.position),
            deck_y: axis(deck.basis.columns[1]),
            deck_z: axis(deck.basis.columns[2]),
            //82DB7598 publishes Motion273 from Processed2468 bit20.
            skateboard_flipped: p.flags_2468 & (1 << 20) != 0,
        },
        board_present: physical.off_board.flag_311 != 0,
        physical_28_byte75: graph_offboard,
        time_since_teleport: skater.animation.motion.riding.time_since_teleport,
    };
    // Fill825999F0/8259C260 publishes discrete off-board inputs before the
    // stock ActionGraph. DerivedControllerInput has already advanced once.
    let mut action_intents = controls.action_intents.clone();
    for intent in skate3_core::input::grind_intentions::produce(&controls.controller) {
        action_intents.insert(intent.name, intent.value);
    }
    // Fill8259AFFC..B088 emits the four analog AG inputs every frame.
    // Stock AG maps OB_Mag to OB_SteerMagnitude; MG attaches that as ob_Mag
    // and attaches OB_BipedWorldX/Z unchanged. ProcessOutput82DB7288..729C
    // publishes the shared Biped708/400 contact gate/direction as333/288.
    let contact = &skater.biped_ground.controller.state.contact;
    for intent in skate3_core::input::offboard_intentions::produce_analog(
        &controls.controller,
        skate3_core::input::offboard_intentions::AnalogObservation {
            effective_skeleton_z: effective_z,
            biped_correction: contact.active.then_some(contact.direction),
        },
    ) {
        action_intents.insert(intent.name, intent.value);
    }
    for intent in skate3_core::input::offboard_intentions::produce_discrete(
        &controls.controller,
        controls.actor_flags,
        physical.air.use_air_reckoning_452 != 0,
    ) {
        action_intents.insert(intent.name, intent.value);
    }
    let pushing = action_intents.contains_key("NewPush") || action_intents.contains_key("Pushing");
    let no_input = skate3_core::input::riding_intentions::no_input(
        &controls.controller,
        pushing,
        physical.air.use_air_reckoning_452 != 0,
    );
    if no_input {
        action_intents.insert("NoInput", 1.0);
    }
    // The S2 Actor's scripted pin applies after every S3 Fill producer,
    // including offboard and grind inputs appended above. Filtering only the
    // initial riding map would let a countdown's Y press dismount the skater.
    #[cfg(feature = "skate3-runtime")]
    controls.host_actor_input.apply_map(&mut action_intents,skater.skeleton_collision.is_ragdoll);
    let mut output = AnimationPhaseOutput::new();
    skater.animation.motion.animation.host_offboard_reset = manual_board_target == Some(false);
    let advanced = skater.animation.advance(
        graphs,
        physics.settings.step.simulation.time_step,
        &action_intents,
        observations,
        &mut output.reset,
    );
    skater.animation.motion.animation.host_offboard_reset = false;
    skater.player_providing_input = !no_input;
    advanced?;
    Ok(output)
}

/// Run once after completed physical output, before camera consumes that output.
/// Animation reads this same stored result on the following actor phase.
pub(crate) fn publish_feedback(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
) -> PhysicalFeedback {
    // World8275F430 -> AirCollector82DA7888 publishes the complete mask.
    // This host runs active gameplay with HoM and challenges outside its scope.
    skater.player_input.physical.scoring.capabilities_204 =
        skate3_core::player::conditioner_capabilities::ConditionerCapabilityContext {
            in_front_end: false,
            hall_of_meat_enabled: false,
            challenge_query_active: false,
            challenge_configuration_enabled: false,
        }
        .capabilities();
    let p = &skater.player_input.processed;
    let deck = physics.board.part_transforms()[BodyId::Deck.index()];
    // CalcLandingQuality82DE61D0 retains its conditioner fields on nonlanding
    // frames. They are not part of ProcessOutput's per-frame packet reset:
    // the delayed air reward must still see the completed landing analysis.
    if let Some(filtered) = skater.player_state.filtered_output.as_ref() {
        skater.landing_quality.update(
            skate3_core::animation::landing_quality::Input {
                previous_filtered_state: filtered.previous_category as u32,
                filtered_state: filtered.category as u32,
                ground_normal: vec4(physics.riding.ground.overall_normal),
                deck_velocity: vec4(physics.riding.motion.linear_velocity),
                flipped: p.flags_2468 & (1 << 20) != 0,
                // Board Fill82C02AD8..2B1C publishes physical part6 Z to
                // bundle0+32; this is separate from Reckoning752's ground frame.
                reckoning_forward: axis(deck.basis.columns[2]),
                // Ground Enter clears this owner before publication, as in82D375C8.
                air_spin: skater.air_reckoning.state.spin_speed,
            },
            &skater.landing_quality_settings,
        );
    }
    let feedback = skater.animation_feedback.update(
        &physics.riding.motion,
        &skater.ground.pumping,
        &skater.ground.wobble,
        ReckoningFeedback {
            // Skeleton16144 -> PhysOutSystemReckoning64,82BE1D84/8C.
            system_position: vec3(skater.animated_skeleton.board_frames.centre_of_mass),
            system_up: physics.riding.reckoning.up,
            board_position: deck.translation,
            target_lean_angle: physics.riding.reckoning_frames.target_lean_angle,
        },
        ControlFeedback {
            processed_flags: p.flags_2468,
            turn: skater.animation_input.fields.turn,
            animation_mirrored: skater.animation.packet.mirrored,
        },
        ground_acceleration::Input {
            deck: super::solve::deck_frame(&physics.board),
            ground: physics.riding.reckoning_frames.ground,
            world_acceleration: vec4(physics.riding.ground.accelerations[BodyId::Deck.index()]),
        },
        physics.riding.reckoning_frames.lateral_tilt,
    );
    skater.physical_feedback = feedback;
    feedback
}

/// PhysOutAnimation reset82DEFAA0 and the initial represented body outputs.
pub(crate) fn initial_feedback() -> PhysicalFeedback {
    PhysicalFeedback {
        turning: skate3_core::input::set_turning::Physical {
            field_32: 0.0,
            field_36: 0.0,
            field_52: 0.0,
            field_56: 0.0,
            field_60: 0.0,
            body_168: 0.0,
        },
        crouching: skate3_core::animation::crouching::Physical {
            body_84: 0.0,
            body_164: 0.0,
            body_188: 0.0,
            force_516: 0.0,
            ground_force_520: 0.0,
            minimum_crouch_528: 0.0,
            deck_angle_532: 0.0,
            animation_height_72: 0.0,
        },
        pumping_acceleration: 0.0,
        ground_acceleration: [0.0; 4],
        // Animation reset vector112 is zero; stock bumps threshold is positive100.
        bumped: false,
        conditioned_turn: [0.0; 8],
    }
}
fn vec3(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn vec4(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}
fn axis(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}
