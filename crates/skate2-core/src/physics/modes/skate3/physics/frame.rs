//! One production tick: controller graphs, physical input/state, shared solve,
//! physical animation output and the normal gameplay camera.
use super::{
    Skate3PhysicsState, PlayerControls, Skate3ControllerState, animation_phase, camera_output, ground_phase,
    input_phase, player_state, solve,
};
use crate::skate3_native::{camera::CameraRuntime, graph_runtime::StockGraphs};
use skate3_core::{
    input::controller::ActionMap,
    math::Vector3,
    physics::{
        board::BodyId,
        phase::{PhysicalOutputSnapshot, PhysicsEvent},
    },
};

pub(crate) struct PreparedFrame {
    tick:u64,
    animation:animation_phase::AnimationPhaseOutput,
    skeleton_queries:super::foot_ik_queries::SkeletonLineTests,
}

pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &mut PlayerControls,
    graphs: &StockGraphs,
    actions: &mut dyn ActionMap,
    input_available: bool,
    camera: &mut CameraRuntime,
) -> Result<(), String> {
    if let Some(prepared)=prepare(physics,skater,controls,graphs,camera)? {
        finish(physics,skater,controls,actions,input_available,camera,prepared)?;
    }
    Ok(())
}

pub(super) fn prepare(physics:&mut Skate3PhysicsState,skater:&mut Skate3ControllerState,
    controls:&mut PlayerControls,graphs:&StockGraphs,camera:&mut CameraRuntime)
    ->Result<Option<PreparedFrame>,String> {
    //A preceding tick the host did not follow with the Actor's update.
    super::respawn::drain_unanswered(physics, skater)?;
    let tick = physics.ticks;
    skater.teleported_this_tick = false;
    skater.trajectory.begin_tick(tick);
    super::offboard_audit_trace::stage(tick, "begin", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("begin", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "begin", skater);
    physics.exchange = super::SimulationExchange::new(tick);
    //SimController8285C968 dispatches the preceding tick's camera messages
    //before simulation. Keep End/Begin ordering when both occur in one update.
    for request in camera.simulation_rate_requests.drain(..) {
        if !physics.network_active { physics.clock.apply(request)?; }
    }
    if physics.network_active { physics.clock = super::clock::SimulationClock::default(); }
    if physics.ticks == 0 {
        player_state::initialize(physics, skater)?;
        //Ctor82DB3008 enters Ground without ProcessOutput. Keep the constructed
        //State/FilteredState zeros (82DE3BE8/82DE5588) until the completed tick.
        //Publishing Ground here runs its ForcePhysics Begin before the initial
        //input reset82DB8998, which would immediately clear that mode again.
    }
    if super::climbing::advance(physics, skater, controls, camera)? { return Ok(None); }
    //8285A06C/A080 completes prior trajectory batches, then A0D0 calls
    //Player slot16=82DB3C78 (vtable82328688). Consume with retained input
    //BEFORE this frame's queries, ProcessInput, and state selection.
    super::biped_air::consume_selector(physics, skater)?;
    physics.board.clear_forces();
    //World8275EA20 starts both batches before actor SetUpPhysics. Foot
    //results stay pending while PlayerInput consumes the preceding records.
    physics
        .riding
        .start_wheel_queries(&physics.board, &physics.world)?;
    let skeleton_queries = super::foot_ik_queries::query(&physics.world, &skater.skeleton)?;
    let animation = crate::skate3_native::trace::info_span!("fixed_animation_graphs").in_scope(|| animation_phase::advance(
        physics,
        skater,
        controls,
        graphs,
        &physics.animation_profile,
    ))
    .map_err(|e| format!("Animation tick{}: {e}", physics.ticks))?;
    super::offboard_audit_trace::stage(tick, "animation", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("animation", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "animation", skater);
    Ok(Some(PreparedFrame {tick,animation,skeleton_queries}))
}

pub(super) fn finish(physics:&mut Skate3PhysicsState,skater:&mut Skate3ControllerState,
    controls:&mut PlayerControls,actions:&mut dyn ActionMap,input_available:bool,
    camera:&mut CameraRuntime,prepared:PreparedFrame)->Result<(),String> {
    let PreparedFrame {tick,mut animation,skeleton_queries}=prepared;
    animation.publish(&skater.animation.packet,&physics.animation_profile,controls.actor_flags);
    if let Some(reply)=skater.teleport_state.take_reply(){animation.publish_external_reset(reply);}
    //ForcePhysics Begin82BB2868 writes the live Skeleton16420 mode once.
    //Consume the graph request so a later physical reset can retain its own0.
    if let Some(mode) = skater.animation.motion.riding.force_mode.take() {
        skater.skeleton_input.force_mode = mode.native_value();
    }
    //PlayerUI82898920 remaps the gameplay packet before both controller
    //and physical input consume it. Reuse this tick's sampled mapping.
    let mut simulation_actions = controls.simulation_actions(actions);
    super::shove_probe::update(skater, animation.packet().external_physics_flag_10689);
    let teleported = input_phase::advance(
        physics,
        skater,
        &animation.packet(),
        &mut simulation_actions,
        input_available,
    )?;
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("processed_input", physics, skater);
    skater.ground_settings = skater.ground_profiles.select(skater.player_input.processed.state_variant_index_2528, skater.player_input.processed.surface_mode_2540)?;
    skater.teleported_this_tick = teleported;
    if teleported {
        skater.respawn.reset_measurements();
        #[cfg(any())]
        super::offboard_root_trace::trace(tick, "teleport", skater);
        //82DB93B0..CC: complete pending queries and clear contact history.
        skater.offboard_contact.reset_history();
        // Biped cadence survives ordinary Ground reset, but a checkpoint teleport
        // must not retain an off-board placement from a prior wipeout recovery.
        skater.biped_ground.controller.reset();
        skater.player_state.reset_for_teleport();
        skater.centre_of_mass_filter.reset();
        skater.animation_feedback.reset();
        // PhysicsConditioner Reset82DE3F38, not the ordinary per-frame packet.
        skater.landing_quality = Default::default();
        player_state::enter_after_teleport(physics, skater)?;
        super::offboard::board_manager::runtime::finish_teleport(physics, skater);
        //82DB8E40 calls SetUpNormal after pose reset, state100 and board reset.
        skater.wipeout_state.ragdoll.restore_normal(
            &mut skater.skeleton,
            &mut skater.skeleton_joints,
            &mut skater.skeleton_collision,
            &mut skater.collision_feedback,
        );
    }
    //World8275EC0C ends board queries before PostInput/state selection.
    physics.riding.finish_wheel_queries()?;
    //Complete preceding-state submissions before PostInput82DB573C publishes
    //them. Host execution is synchronous; no current-state submission exists
    //yet, preserving next-tick visibility and later PreState82DB60EC consumption.
    {
        let scene =
            super::offboard::grab_scene::Scene::new(&physics.world, &physics.offboard_grab_scene);
        skater.offboard_grab.execute_queries(&scene)?;
    }
    let state_before_selection = skater.player_state.current();
    let riding = matches!(state_before_selection,
        skate3_core::player::state::PhysicalStateId::PhysicsGround
        | skate3_core::player::state::PhysicalStateId::GroundAnimation
        | skate3_core::player::state::PhysicalStateId::SlideGround
        | skate3_core::player::state::PhysicalStateId::RevertGround);
    let deck_rates = physics.board.bodies()[BodyId::Deck.index()].rates;
    let deck_speed = deck_rates.linear_velocity.x.hypot(deck_rates.linear_velocity.z);
    if teleported {
        physics.puddle_response = Default::default();
    }
    let puddle = if riding && !teleported {
        crate::ui::puddle_physics::contact_at([deck_rates.position.x, deck_rates.position.y, deck_rates.position.z])
    } else {
        None
    };
    physics.puddle_decision = physics.puddle_response.step(puddle.map(|contact| contact.depths), deck_speed,
        physics.settings.step.simulation.time_step);
    player_state::post_input_and_select(physics, skater)?;
    let state_after_selection = skater.player_state.current();
    super::offboard_audit_trace::stage(tick, "selected", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("selected", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "selected", skater);
    if state_before_selection != state_after_selection {
        if skater.player_input.processed.flags_2476 & (1 << 22) != 0
            || state_before_selection == skate3_core::player::state::PhysicalStateId::HandPlant
            || state_after_selection == skate3_core::player::state::PhysicalStateId::HandPlant {
            crate::skate3_native::trace::info!("HANDPLANT_STATE tick={tick} from={state_before_selection:?} to={state_after_selection:?} flags={:08x} phase={} processed={:08x}/{:08x}/{:08x}",
                skater.handplant.flags, skater.handplant.phase,
                skater.player_input.processed.flags_2468, skater.player_input.processed.flags_2476,
                skater.player_input.processed.flags_2480);
        }
        physics.exchange.emit_event(
            tick,
            PhysicsEvent::StateChanged {
                from: state_before_selection,
                to: state_after_selection,
            },
        )?;
    }
    player_state::pre_state(physics, skater)?;
    match skater.player_state.current() {
        skate3_core::player::state::PhysicalStateId::RevertGround => super::revert_state::update(physics,skater)?,
        skate3_core::player::state::PhysicalStateId::HandPlant => super::handplant::update(physics,skater)?,
        skate3_core::player::state::PhysicalStateId::FootPlant => super::footplant::ground::update(physics, skater)?,
        skate3_core::player::state::PhysicalStateId::Boneless => super::boneless::update(physics, skater)?,
        skate3_core::player::state::PhysicalStateId::PhysicsGround => {
            let p = &skater.player_input.processed;
            let com = p.animation_com_to_deck_752.map(f32::from_bits);
            physics.riding.update_ground_reckoning(
                &physics.board,
                super::riding_outputs::RidingPoseInputs {
                    com_to_deck: Vector3::new(com[0], com[1], com[2]),
                    body_spin: skater.animation_input.extra.physical_body_spin,
                },
                p.flags_2468,
                skater.animation_input.fields.balance,
                p.flags_2476 & 0x4000_0000 != 0,
                p,
            );
            #[cfg(debug_assertions)]
            super::dev_trace::checkpoint("ground_reckoning", physics, skater);
            //PhysicsGround::PreUpdate82D37C50 clears the previous push suppression
            //after Reckoning; this frame's propulsion may then set it again.
            skater.ground.state.push_suppressed_2730 = false;
            ground_phase::advance(physics, skater)?;
            super::handplant::ground_update(physics,skater)?;
            input_phase::update_ground(physics, skater)?;
        }
        skate3_core::player::state::PhysicalStateId::PhysicsAirSecondary => {
            super::grind_trick::advance(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::PhysicsAir => {
            super::air_phase::advance(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::KnownAir => {
            super::known_air::update(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::BipedAir => {
            super::biped_air::update(physics, skater, controls)?
        }
        skate3_core::player::state::PhysicalStateId::BipedGround
        | skate3_core::player::state::PhysicalStateId::OffBoardPushing => {
            let contact = skate3_core::player::offboard::ground_job::ContactSnapshot {
                readiness: skater.offboard_contact.readiness() as i32,
                prefix: skater.offboard_contact.prefix(),
            };
            let input = super::biped_ground::update(physics, skater, controls, contact)?;
            //Sync82D32164 submits AFTER Skeleton; both query batches become
            //visible only to their next input/PreUpdate consumer.
            skater.offboard_contact.submit(
                input,
                skater.player_input.processed.actor_query_2952 as i32,
                &super::offboard::contact_toolkit::StaticScene::new(&physics.world)?,
            )?;
            super::biped_ground::submit_geometry(physics, skater)?;
        }
        skate3_core::player::state::PhysicalStateId::GroundAnimation => {
            super::ground_animation::advance(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::LandingOnDeck => {
            super::landing_on_deck::advance(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::SlideGround => {
            super::slide_state::update(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::WipeoutGround => {
            super::wipeout_states::advance(physics, skater)?
        }
        skate3_core::player::state::PhysicalStateId::Teleporting => {
            //VT823272FC slot8=82D431F0; slots12/40 are original empty leaves.
            if skater.teleport_state.update(&skater.player_input.processed) {
                skater.respawn.request();
            }
        }
        state
            if state.is_grind()
                || state == skate3_core::player::state::PhysicalStateId::Nonspecific =>
        {
            super::grind::advance(physics, skater)?
        }
        state => return Err(format!("Selected {state:?} requires its physical update")),
    }
    // Source State82DB6120 submits the queue once, then advances state time.
    // BoardRuntime applies that same queue during its shared solve.
    skater.player_input.player.state_timer_1344 += skater.player_input.processed.timestep_2604;
    super::offboard_audit_trace::stage(tick, "state", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("state", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "state", skater);
    //82DB6150: update the real possession owner after state movement, before solve.
    super::offboard::board_manager::runtime::update(physics, skater);
    super::offboard_audit_trace::stage(tick, "possession", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("board_attachment", physics, skater);
    physics.processed_flags_2468 = skater.player_input.processed.flags_2468;
    //World8275ECA4 ends skeleton tests after state/forces and before solving.
    //Teleport resets previous observations, but preserves this pending batch.
    skeleton_queries.publish(&mut skater.player_input.player);
    skater.hips_ground_start = skeleton_queries.hips.start_position;
    if physics.puddle_decision.drag > 0.0 && riding {
        let inverse_mass = physics.board.bodies()[BodyId::Deck.index()].inertia.inverse_mass;
        if inverse_mass.is_finite() && inverse_mass > 0.0001 {
            let scale = (physics.puddle_decision.drag / inverse_mass).min(100.0);
            physics.board.forces_mut().append(skate3_core::physics::force_queue::QueuedPointForce {
                tag: 20,
                force_world: Vector3::new(-deck_rates.linear_velocity.x * scale, 0.0, -deck_rates.linear_velocity.z * scale),
                point_body: Vector3::ZERO,
            });
        }
    }
    crate::skate3_native::trace::info_span!("fixed_collision_and_solve").in_scope(|| solve::advance(physics, skater, skater.ground.steering.targets))?;
    // After selection cleared the earlier requests, as the post-physics wipeout checks do.
    if physics.puddle_decision.wipeout {
        skater.wipeout.state.request(12, deck_speed);
    }
    super::offboard_audit_trace::stage(tick, "solve", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("solve", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "solve", skater);
    //ProcessOutput82DB6EE8 resets the packet before its component publishers.
    //All consumers of the preceding output have completed this frame's input.
    super::player_input::reset_outputs(&mut skater.player_input.physical);
    crate::skate3_native::trace::info_span!("fixed_finish_skater").in_scope(|| physics.finish_skater(skater))?;
    super::offboard_audit_trace::stage(tick, "finish", physics, skater, controls);
    #[cfg(debug_assertions)]
    super::dev_trace::checkpoint("finish", physics, skater);
    #[cfg(any())]
    super::offboard_root_trace::trace(tick, "finish", skater);
    let simulation = physics.settings.step.simulation;
    skater
        .player_input
        .update_dynamic_normal(&physics.riding, simulation.gravity_acceleration);
    skater.player_input.publish_board(&physics.riding)?;
    skater.player_input.physical.skeleton.publish_deck_angles(
        skater.animated_skeleton.record.pose[0][2],
        skater.animation.packet.board_flipped,
    );
    let up = physics.riding.reckoning.up;
    skater.player_input.publish_grind_graph_outputs(
        &skater.skeleton.record,
        [up.x, up.y, up.z, 0.0],
        &skater.trajectory.selector,
    )?;
    player_state::publish(physics, skater)?;
    skater
        .grind_camera
        .condition_fields(&mut skater.player_input.physical.grinds);
    let physical = &skater.player_input.physical;
    skater.centre_of_mass_output = skater.centre_of_mass_filter.update(
        physical.reckoning.vector_64.map(f32::from_bits),
        physical.reckoning.vector_16.map(f32::from_bits),
    );
    let feedback = animation_phase::publish_feedback(physics, skater);
    super::respawn::observe(physics, skater)?;
    let deck = physics.board.bodies()[BodyId::Deck.index()];
    let rider = skater
        .skeleton
        .bodies()
        .first()
        .ok_or("Physical output requires the constructed rider body")?;
    let ground_normal = physics.riding.ground.wheel_normal;
    let predicted = skater.player_input.physical.collision.predicted_position_64;
    let predicted_position = Vector3::new(
        f32::from_bits(predicted[0]),
        f32::from_bits(predicted[1]),
        f32::from_bits(predicted[2]),
    );
    let events = physics.exchange.events().to_vec();
    let landed = events.iter().any(|event| matches!(event, PhysicsEvent::Landing));
    physics.exchange.publish_output(PhysicalOutputSnapshot {
        tick,
        state: skater.player_state.current(),
        board_position: deck.rates.position,
        board_linear_velocity: deck.rates.linear_velocity,
        rider_root_position: rider.rates.position,
        rider_linear_velocity: rider.rates.linear_velocity,
        ground_normal,
        contact_count: physics.riding.ground.part_contact_count as u32,
        predicted_position,
        grounded: matches!(
            skater.player_state.current(),
            skate3_core::player::state::PhysicalStateId::PhysicsGround
        ),
        wiping_out: skater.skeleton_collision.is_ragdoll,
        landed,
        events,
    });
    // SKATE2_TRACE_SKATE3_PUSH_TICKS: every tick, not only while the Pushing intent is held.
    static PUSH_TICKS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if controls.action_intents.contains_key("Pushing")
        || *PUSH_TICKS.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_PUSH_TICKS").is_some())
    {
        crate::diagnostics::test_log::event("skate3_push_cycle",format_args!(
            "tick={tick} state={} left={} right={} held_time={:?} push={:?} speed={} target={}",
            skater.player_state.current() as u32,controls.action_intents.contains_key("LeftPush"),
            controls.action_intents.contains_key("RightPush"),skater.animation.motion.animation.motion_intents.get("Pushing"),
            skater.animation.motion.push_state,physics.riding.motion.ground_speed,skater.animation_input.contacts.push_speed));
    }
    if teleported { camera.reset_after_teleport(); }
    camera_output::advance(physics, skater, &feedback, camera)?;
    super::climbing::approach::advance(physics, skater, controls);
    skater.animation_input.finish_output_publication();
    skater.player_input.player.update_count_1316 =
        skater.player_input.player.update_count_1316.wrapping_add(1);
    super::shove_probe::ShoveProbe::capture(skater);
    physics.clock.finish_tick();
    Ok(())
}
