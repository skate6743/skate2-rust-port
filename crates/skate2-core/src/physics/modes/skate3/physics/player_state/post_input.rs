//! Original PostInput82DB5588 with the live ballistic trajectory selector.
//! The authored world has no grind edges or grab-spline descriptors.
use super::*;
use skate3_core::player::post_input::*;

pub(crate) struct PostInputState {
    pub jump_reference: [u32; 4],
    pub jump_fix_frames: u32,
    pub latch_frames: u32,
    pub state_frames: u32,
    pub heading_adjust: f32,
    pub complete: bool,
    pub trajectory_pending: bool,
    pub trajectory_valid: bool,
    pub trajectory_available: bool,
    pub trajectory_new_candidate: bool,
}
impl PostInputState {
    pub fn new() -> Self {
        Self {
            //Original Player ctor82DB1AAC=1000,1ACC=0,3028 clears1264.
            jump_reference: [0; 4],
            jump_fix_frames: 1000,
            latch_frames: 0,
            state_frames: 100,
            heading_adjust: 0.0,
            complete: false,
            trajectory_pending: false,
            trajectory_valid: false,
            trajectory_available: false,
            trajectory_new_candidate: false,
        }
    }
}
struct Services<'a> {
    heading: f32,
    grind: &'a mut super::super::player_input::grind::GrindInputState,
    pending: Option<super::super::player_input::grind::Pending>,
    grind_context: super::super::player_input::grind::PostContext,
    host: super::super::grind_host::LiveHost<'a>,
    result: Option<super::super::player_input::grind::PostResult>,
    processed: &'a mut skate3_core::player::input_phase::ProcessedPhysicsInput,
    trajectory: &'a mut super::super::air_trajectory::AirTrajectoryRuntime,
    trajectory_input: skate3_core::air::trajectory::SelectorInput,
    trajectory_board_position: [f32; 4],
    world: &'a skate3_core::physics::board_world::BoardWorld,
    grab_records: [Option<skate3_core::player::offboard::grab_scene::Record>; 2],
    error: Option<String>,
}
impl PostInputServices for Services<'_> {
    fn update_grind_manager_82d8ab08(&mut self) {
        let Some(pending) = self.pending.take() else {
            self.error = Some("Grind post-input requires this tick's pre-input queries".into());
            return;
        };
        match self.grind.post_update(
            self.processed,
            self.world,
            pending,
            self.grind_context,
            &mut self.host,
        ) {
            Ok(result) => self.result = Some(result),
            Err(error) => self.error = Some(error),
        }
    }
    fn update_trajectory_selector_82d68800(&mut self) -> u8 {
        //82DB56B0 runs the real selector after GrindManager. That manager
        //can change2476, so use its current output rather than a prior snapshot.
        let mut input = self.trajectory_input;
        input.flags_2476 = self.processed.flags_2476;
        let grind_context = super::super::air_trajectory::GrindContext::from_processed(
            self.processed,
            self.trajectory_board_position,
        );
        match self.trajectory.update(input, self.world, grind_context) {
            Ok(valid) => u8::from(valid),
            Err(error) => {
                self.error = Some(error);
                0
            }
        }
    }
    fn calculate_scalar_2740_82db5e10(&mut self) -> f32 {
        self.heading
    }
    fn register_candidate_82762ab0(&mut self, registration: CandidateRegistration) {
        let index = match registration {
            CandidateRegistration::First1888 => 0,
            CandidateRegistration::Second2176 => 1,
        };
        match &self.grab_records[index] {
            Some(record) => copy_grab_record_82762ab0(
                &mut self.processed.grab_records_1888_2176[index],
                &record.0,
            ),
            None => {
                self.error = Some("Grab publication requested without a completed record".into())
            }
        }
    }
}
pub(super) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let trajectory_input = super::super::air_phase::selector_input(physics, skater)?;
    let trajectory_board_position = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Post-input trajectory requires this tick's board toolkit")?
        .deck[3];
    let post = &mut skater.player_state.post;
    let p = &mut skater.player_input.processed;
    //82DB573C ->82D740F8 consumes the one live owner's completed results.
    let publication = skater.offboard_grab.publish();
    let mut candidates = CandidatePublicationFields {
        first_object_present_196: publication.records[0].is_some(),
        first_pending_288: publication.records[0].is_some(),
        second_object_present_500: publication.records[1].is_some(),
        second_pending_592: publication.records[1].is_some(),
        staged_word_12768: publication.object.flatten().unwrap_or(p.object_2464),
        staged_valid_12772: u32::from(publication.object.flatten().is_some()),
        staged_latched_12776: u32::from(skater.offboard_grab.interactable_latched),
        staged_pending_12780: publication.object.is_some(),
    };
    let mut player = PostInputPlayerFields {
        jump_reference_1264: post.jump_reference,
        flags_1296: skater.player_input.player.flags_1296,
        state_frames_1304: skater.player_input.player.ground_history_frames_1304 as u32,
        jump_fix_frames_1308: post.jump_fix_frames,
        latch_frames_1320: post.latch_frames,
    };
    let mut processed = PostInputProcessedFields {
        jump_reference_848: post.jump_reference,
        word_2464: p.object_2464,
        flags_2468: p.flags_2468,
        flags_2472: p.flags_2472,
        flags_2480: p.flags_2480,
        flags_2484: p.flags_2484,
        current_state_2508: p.state_2508,
        state_frames_2572: post.state_frames,
        jump_fix_frames_2576: post.jump_fix_frames,
        scalar_2740: post.heading_adjust,
    };
    let mut output = PostInputPhysOutFields {
        reset_state_frames_316: skater
            .player_state
            .ground_output
            .as_ref()
            .is_some_and(|g| g.ground_32.wall_ride_exit),
        //82DB5588 consumes the preceding state's actual jump publication.
        capture_jump_reference_442: skater.player_input.physical.air.launched_442 != 0,
        jump_reference_128: skater.player_input.physical.air.launch_velocity_128,
        complete_76: post.complete,
    };
    let heading = physics.riding.update_input_heading(
        f32::from_bits(p.vectors_464_480_496_512_528[0][1]),
        p.scalar_2612,
    );
    let mut services = Services {
        heading,
        grind: &mut skater.player_input.grind,
        pending: skater.player_input.pending_grind.take(),
        grind_context: super::super::player_input::grind::PostContext {
            board: super::super::solve::deck_frame(&physics.board),
            balance_2720: skater.animation_input.fields.balance,
            translation_2796: skater.animation_input.extra.grind_translation,
            stability_nudge_2800: skater.animation_input.extra.grind_stability_nudge,
            up_down_2804: skater.animation_input.extra.grind_up_down,
            grab_min_height_2808: skater.animation_input.extra.grind_grab_min_height,
        },
        host: super::super::grind_host::LiveHost {
            board: &mut physics.board,
            settings: &mut physics.settings,
            materials: &physics.grind_materials,
        },
        result: None,
        processed: p,
        trajectory: &mut skater.trajectory,
        trajectory_input,
        trajectory_board_position,
        world: &physics.world,
        grab_records: publication.records,
        error: None,
    };
    run_post_input(
        PostInputContext {
            player: &mut player,
            processed: &mut processed,
            phys_out: &mut output,
            candidates: &mut candidates,
        },
        &mut services,
    );
    if let Some(error) = services.error.take() {
        return Err(error);
    }
    let grind_result = services
        .result
        .take()
        .ok_or("Grind post-input did not publish its result")?;
    processed.flags_2468 |= services.processed.flags_2468 & 0x0004_0000;
    drop(services);
    for reason in grind_result.wipeout_reasons {
        skater.wipeout.state.request(reason, 0.0);
    }
    skater.grind.observe(grind_result.observation);
    skater.player_input.grind_observation = Some(grind_result.observation);
    let g = p.grind;
    skater.ground_lifecycle.edge = (g.flags_1516 & 0x0800_0000 != 0).then(|| {
        let xyz = |v: [u32; 4]| {
            skate3_core::math::Vector3::new(
                f32::from_bits(v[0]),
                f32::from_bits(v[1]),
                f32::from_bits(v[2]),
            )
        };
        super::super::ground_phase::GroundEdge {
            flags: g.flags_1516,
            point: g.vector_1232.map(f32::from_bits),
            start: xyz(g.second_start_1312),
            end: xyz(g.second_end_1328),
        }
    });
    //These are observations of the actual owner, never inputs to selection.
    post.trajectory_pending = skater.trajectory.selector.pending();
    post.trajectory_valid = skater.trajectory.selector.valid();
    post.trajectory_available = skater.trajectory.selector.valid();
    post.trajectory_new_candidate = skater.trajectory.selector.just_changed();
    skater.player_input.player.flags_1296 = player.flags_1296;
    skater.player_input.player.ground_history_frames_1304 = player.state_frames_1304 as i32;
    p.flags_2468 = processed.flags_2468;
    p.flags_2472 = processed.flags_2472;
    p.flags_2480 = processed.flags_2480;
    p.flags_2484 = processed.flags_2484;
    p.object_2464 = processed.word_2464;
    post.jump_reference = player.jump_reference_1264;
    post.jump_fix_frames = player.jump_fix_frames_1308;
    post.latch_frames = player.latch_frames_1320;
    post.state_frames = processed.state_frames_2572;
    post.heading_adjust = processed.scalar_2740;
    post.complete = output.complete_76;
    Ok(())
}
