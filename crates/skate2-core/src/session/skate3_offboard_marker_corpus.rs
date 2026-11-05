{
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.actor.try_set_pinned(false).unwrap();
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
    }).unwrap();
    let mut saved = None;
    let mut saved_foot = 0;
    let mut walk_start = None;
    let mut walk_end = None;
    let mut returned = false;
    for step in 0..460 {
        if step == 240 {
            session.try_with_selected_queued_frame(|frame, _, _| {
                frame.actor.try_spawn_foot_forward_stance(saved_foot).unwrap();
                frame.actor.try_publish_teleport(&saved.expect("offboard marker captured"), false).unwrap();
            }).unwrap();
        }
        let sample = crate::input_pad::XboxPadSample {
            buttons: if step == 40 { 1 << 15 } else { 0 },
            left_y: if (150..220).contains(&step) || step >= 310 { i16::MAX } else { 0 },
            ..neutral
        };
        pad.scripted_controller_zero = Some(sample);
        let input = session.bound_input.as_mut().unwrap();
        input.poll_and_cache(pad).unwrap();
        assert!(input.try_preupdate_enqueue(pad,
            &session.player.as_ref().unwrap().selected.owners.loader, true, true, &sync).unwrap());
        let raw = input.raw_controller();
        session.try_with_selected_queued_frame(|mut frame, ground, queued| {
            frame.tick.configure_skate3(settings, raw);
            frame.try_step_queued::<_, _, _, _, _, _, Infallible>(SelectedTickRequest {
                input: queued, dt: 1. / 60., lod: 0, wait_for_bs_job: true,
                physics: SkateboardGroundFrameInput {
                    schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                    ground_pool: ground, ground_group: 0, pipeline_culling: None,
                    allow_multiple_writes: true, spy_flags: 4,
                },
                pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                post_grind: GrindManagerFillPolicy::RequireNative,
            }, &mut numeric, &mut sync, &mut recognizer, &mut feedback, &mut clamp,
               &mut HostReckoningNumeric, None, None).unwrap();
            let output = frame.tick.skate3_output().unwrap();
            if step == 120 {
                assert!(output.offboard, "fixture must place its marker while on foot");
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let native = physical.owner().phys_out().lock().unwrap();
                saved = Some(crate::actor_construction::breadcrumbs::calc_respawn_transform::try_calc_respawn_transform(
                    &native, &cvars, &mut HostReckoningNumeric).unwrap());
                saved_foot = u32::from(!frame.actor.try_current_foot_forward_with_fakie().unwrap());
            }
            if (240..310).contains(&step) {
                // Native reset acknowledgment can precede the alternate
                // solver's exported pose. Observe arrival before checking
                // stability, and require that arrival before walking starts.
                let pending = frame.actor.try_skate3_teleport_publication().unwrap().3;
                let target = saved.unwrap().known_lanes().unwrap();
                let distance = (0..3).map(|axis| (output.root_transform[3][axis] - target[12 + axis]).powi(2)).sum::<f32>().sqrt();
                if returned {
                    assert!(distance < 5., "marker return left its destination: step={step} distance={distance}");
                }
                returned |= !pending && output.offboard && distance < 5.;
                if step == 309 { assert!(!pending && returned, "offboard marker return did not finish before walking"); }
            }
            if step == 310 { walk_start = Some(output.root_transform[3]); }
            if step == 459 { walk_end = Some(output.root_transform[3]); }
            if step % 30 == 0 || (240..250).contains(&step) {
                eprintln!("offboard marker step={step} pending={} {}",
                    frame.actor.try_skate3_teleport_publication().unwrap().3,
                    frame.tick.skate3_offboard_motion_trace());
            }
            assert!(output.root_transform.iter().flatten().all(|v| v.is_finite()));
        }).unwrap();
    }
    assert!(returned, "offboard marker did not acknowledge its completed return");
    let start = walk_start.unwrap();
    let end = walk_end.unwrap();
    let distance = [0, 2].into_iter().map(|axis| (end[axis] - start[axis]).powi(2)).sum::<f32>().sqrt();
    assert!(distance > 0.5, "walking after offboard marker return is stuck: distance={distance} start={start:?} end={end:?}");
    eprintln!("offboard marker walk-away distance={distance}");
}
