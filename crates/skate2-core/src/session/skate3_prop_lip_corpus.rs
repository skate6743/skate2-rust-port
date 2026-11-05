{
    // A large kicker spawned as an ordinary living-world DMO (the online static-DMO
    // path) ahead of the settled skater; Skate 3 mode pushes up it. Box/capsule seams
    // under its mesh used to stop a Skate 3 wheel at the toe (the "lip").
    // PROP_LIP_NATIVE: Skate 2 physics for comparison.
    const KICKER_LARGE: u64 = 0x0001_92e4_03e3_8707;
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.actor.try_set_pinned(false).unwrap();
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
    }).unwrap();
    let skate3 = std::env::var_os("PROP_LIP_NATIVE").is_none();
    let settings = if skate3 { settings } else { GameplaySettings { skate3: Default::default(), ..settings } };
    let root_lanes = |root: &crate::skeleton_root_transforms::SkeletonRootMatrix| -> [f32; 16] {
        let mut lanes = [0.0; 16];
        for (column, vector) in root.columns().iter().enumerate() {
            lanes[column * 4..column * 4 + 3].copy_from_slice(&vector.xyz);
        }
        lanes[15] = 1.0;
        lanes
    };
    let mut start_y = None;
    let mut highest = f32::MIN;
    let mut settled = 0;
    let mut spawned_at = None;
    let mut mesh_bodies: Option<usize> = None;
    for step in 0..900 {
        if spawned_at.is_none() && settled >= 30 {
            spawned_at = Some(step);
            let spawned = session.try_with_selected_queued_frame(|frame, _, _| {
                let root = {
                    let physical = frame.actor.cached_physical_allocation().unwrap();
                    let output = physical.owner().phys_out().lock().unwrap();
                    root_lanes(&output.try_skeleton_output().unwrap().skeleton_root_matrix)
                };
                let length = root[8].hypot(root[10]);
                let forward = [root[8] / length, 0.0, root[10] / length];
                let right = [forward[2], 0.0, -forward[0]];
                // 7 m ahead; the model origin is the kicker's side edge (ramp spans
                // local x 0..1.86 m), so centre it on the skater's line.
                let mut transform = [
                    right[0], 0.0, right[2], 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    forward[0], 0.0, forward[2], 0.0,
                    root[12] + forward[0] * 7.0 - right[0] * 0.93, 0.0,
                    root[14] + forward[2] * 7.0 - right[2] * 0.93, 1.0,
                ];
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let player = physical.owner().skate3.lock().unwrap();
                transform[13] = match player.as_ref() {
                    Some(player) => player.test_ground_height([transform[12], root[13] + 1.0, transform[14]]).unwrap(),
                    None => root[13] - 0.15,
                };
                drop(player);
                drop(physical);
                eprintln!("prop lip kicker at {:?} skater root {:?}", &transform[12..15], &root[12..15]);
                let record = skate2_formats::pegasus::Dmo {
                    transform, bounding_box_min: [0.0; 4], bounding_box_max: [0.0; 4],
                    guid: 0x5052_4f50_4c49_5000, local_guid: 0, instance_guid: KICKER_LARGE,
                    serialized_instance_word: 0, name_offset: 0, name: "prop lip kicker".into(),
                };
                let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
                living.try_spawn_online_dmos(frame.world, frame.actor, frame.cvars, recipes, &[record],
                    &crate::challenge::invisible_boundary::InvisibleBoundaries::default()).unwrap()
            }).unwrap();
            assert_eq!(spawned.len(), 1, "the kicker would not spawn");
        }
        let sample = crate::input_pad::XboxPadSample {
            buttons: if spawned_at.is_some_and(|at| [10, 35, 60, 85, 110, 135, 160, 185, 210, 235, 260, 285].iter().any(|d| (at + d..at + d + 4).contains(&step))) { 0x1000 } else { 0 },
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
            let (state, board, speed) = {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let native = physical.owner().phys_out().lock().unwrap();
                (native.try_state_output().unwrap().physics_state,
                    native.try_camera_board_output().unwrap().deck_position.xyz,
                    native.try_system_centre_and_damped_velocity().unwrap().1.xyz)
            };
            let y = board[1];
            let Some(at) = spawned_at else {
                if state == 100 { settled += 1; } else { settled = 0; }
                return;
            };
            if start_y.is_none() { start_y = Some(y); }
            if step > at + 420 { return; }
            if state != 300 { highest = highest.max(y); }
            if skate3 && step == at + 5 {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                mesh_bodies = physical.owner().skate3.lock().unwrap().as_ref().map(|p| p.test_prop_world_counts().0);
            }
            if step % 15 == 0 {
                eprintln!("prop lip step={step} state={state} board={board:?} speed={:.2}", speed[0].hypot(speed[2]));
            }
        }).unwrap();
    }
    let rise = highest - start_y.expect("the skater never settled");
    let mode = if skate3 { "skate3" } else { "skate2" };
    eprintln!("prop lip {mode} RESULT board rose {rise:.3} m over the kicker; mesh bodies {mesh_bodies:?}");
    assert!(rise > 0.6, "prop lip {mode}: the board did not ride up the kicker (rose {rise:.3} m)");
    return;
}
