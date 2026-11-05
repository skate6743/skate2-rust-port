{
    // An on-foot marker return with the stick left alone must not move off.
    // `walking`: the stick is held until the return, so any carried-over
    // motion shows up; otherwise the skater stood still the whole time.
    let walking = matches!(trick_probe, Some(PlayerProbe::OffboardMarkerDrift(true)));
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.actor.try_set_pinned(false).unwrap();
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
    }).unwrap();
    let mut saved = None;
    let mut saved_foot = 0;
    let mut settled_at = None;
    let mut worst = (0.0_f32, 0);
    for step in 0..480 {
        if step == 240 {
            session.try_with_selected_queued_frame(|frame, _, _| {
                frame.actor.try_spawn_foot_forward_stance(saved_foot).unwrap();
                frame.actor.try_publish_teleport(&saved.expect("offboard marker captured"), false).unwrap();
            }).unwrap();
        }
        let sample = crate::input_pad::XboxPadSample {
            buttons: if step == 40 { 1 << 15 } else { 0 },
            left_y: if walking && (150..240).contains(&step) { i16::MAX } else { 0 },
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
            if step >= 240 {
                let pending = frame.actor.try_skate3_teleport_publication().unwrap().3;
                let target = saved.unwrap().known_lanes().unwrap();
                let root = output.root_transform[3];
                let off = (root[0] - target[12]).hypot(root[2] - target[14]);
                let speed = output.root_velocity[0].hypot(output.root_velocity[2]);
                if settled_at.is_none() && !pending && output.offboard {
                    settled_at = Some((step, off));
                }
                if let Some((_, start)) = settled_at {
                    let drift = (off - start).abs();
                    if drift > worst.0 { worst = (drift, step); }
                }
                if step < 262 || step % 15 == 0 {
                    eprintln!("drift walking={walking} step={step} pending={pending} state={} offboard={} off_marker={off:.3} speed={speed:.3} velocity={:?}",
                        output.state, output.offboard, output.root_velocity);
                }
            }
        }).unwrap();
    }
    let (settled, start) = settled_at.expect("offboard marker return never settled on foot");
    eprintln!("drift walking={walking} RESULT settled_step={settled} start_off_marker={start:.3} worst_drift={:.3} at step {}", worst.0, worst.1);
    assert!(worst.0 < 0.25, "an on-foot marker return moved off on its own: {:.3} m", worst.0);
}
