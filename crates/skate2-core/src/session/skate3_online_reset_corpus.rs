{
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
        frame.actor.try_set_pinned(false).unwrap();
        frame.actor.try_set_pinned_gestures(true).unwrap();
    }).unwrap();
    let mut expected = None;
    let mut completed = false;
    let mut saw_previous_pending = false;
    let mut destination = test_spawn;
    destination[12] += 2.;
    for step in 0..140 {
        if step == 20 || step == 23 {
            session.try_with_selected_queued_frame(|frame, _, _| {
                frame.actor.try_set_pinned(true).unwrap();
                let physical = frame.actor.cached_physical_allocation().unwrap();
                if step == 23 {
                    saw_previous_pending = physical.owner().skate3.lock().unwrap().as_ref().unwrap().teleport_pending();
                }
                let target = if step == 20 { test_spawn } else { destination };
                frame.actor.try_publish_teleport(&crate::skeleton_root_transforms::SkeletonRootMatrix::known(target), step != 20).unwrap();
                if step == 23 { expected = Some(frame.actor.try_teleport_publication().unwrap().0); }
            }).unwrap();
        }
        pad.scripted_controller_zero = Some(neutral);
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
                }, pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                post_grind: GrindManagerFillPolicy::RequireNative,
            }, &mut numeric, &mut sync, &mut recognizer, &mut feedback, &mut clamp,
                &mut HostReckoningNumeric, None, None).unwrap();
            let physical = frame.actor.cached_physical_allocation().unwrap();
            let output = physical.owner().phys_out().lock().unwrap();
            if step >= 23 {
                let (generation, _) = frame.actor.try_teleport_publication().unwrap();
                completed |= Some(generation) == expected && output.try_completed_teleport().unwrap()
                    .is_some_and(|matrix| matrix.known_lanes() == Some(destination));
            }
            if step % 10 == 0 || (20..30).contains(&step) {
                eprintln!("online reset supersession step={step} expected={expected:?} completed={completed} pending={}",
                    physical.owner().skate3.lock().unwrap().as_ref().unwrap().teleport_pending());
            }
        }).unwrap();
    }
    assert!(saw_previous_pending, "fixture did not retain the prior offboard continuation");
    assert!(completed, "the newer onboard map reset stayed behind the offboard continuation");
    return;
}
