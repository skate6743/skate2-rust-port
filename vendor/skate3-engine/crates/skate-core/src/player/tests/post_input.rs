use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Grind,
    Trajectory,
    Scalar,
    Register(CandidateRegistration),
}

struct Services {
    events: Vec<Event>,
    trajectory: u8,
    scalar: f32,
}

impl PostInputServices for Services {
    fn update_grind_manager_82d8ab08(&mut self) {
        self.events.push(Event::Grind);
    }

    fn update_trajectory_selector_82d68800(&mut self) -> u8 {
        self.events.push(Event::Trajectory);
        self.trajectory
    }

    fn calculate_scalar_2740_82db5e10(&mut self) -> f32 {
        self.events.push(Event::Scalar);
        self.scalar
    }

    fn register_candidate_82762ab0(&mut self, registration: CandidateRegistration) {
        self.events.push(Event::Register(registration));
    }
}

fn player() -> PostInputPlayerFields {
    PostInputPlayerFields {
        jump_reference_1264: [1, 2, 3, 4],
        flags_1296: 0,
        state_frames_1304: 4,
        jump_fix_frames_1308: 8,
        latch_frames_1320: 0,
    }
}

fn processed() -> PostInputProcessedFields {
    PostInputProcessedFields {
        jump_reference_848: [0; 4],
        word_2464: 0,
        flags_2468: 0,
        flags_2472: 0,
        flags_2480: 0,
        flags_2484: 0,
        current_state_2508: 100,
        state_frames_2572: 0,
        jump_fix_frames_2576: 0,
        scalar_2740: 0.0,
    }
}

fn phys_out() -> PostInputPhysOutFields {
    PostInputPhysOutFields {
        reset_state_frames_316: false,
        capture_jump_reference_442: false,
        jump_reference_128: [10, 11, 12, 13],
        complete_76: false,
    }
}

fn candidates() -> CandidatePublicationFields {
    CandidatePublicationFields {
        first_object_present_196: false,
        first_pending_288: false,
        second_object_present_500: false,
        second_pending_592: false,
        staged_word_12768: 0,
        staged_valid_12772: 0,
        staged_latched_12776: 7,
        staged_pending_12780: false,
    }
}

#[test]
fn phase_preserves_service_order_and_native_counter_publication() {
    let mut player = player();
    let mut processed = processed();
    let mut output = phys_out();
    output.reset_state_frames_316 = true;
    output.capture_jump_reference_442 = true;
    let mut candidates = candidates();
    candidates.first_object_present_196 = true;
    candidates.first_pending_288 = true;
    candidates.second_object_present_500 = true;
    candidates.second_pending_592 = true;
    let mut services = Services {
        events: Vec::new(),
        trajectory: 3,
        scalar: f32::from_bits(0x3E80_0000),
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

    assert_eq!(
        services.events,
        [
            Event::Grind,
            Event::Trajectory,
            Event::Scalar,
            Event::Register(CandidateRegistration::First1888),
            Event::Register(CandidateRegistration::Second2176),
        ]
    );
    assert_eq!(player.state_frames_1304, 1);
    assert_eq!(processed.state_frames_2572, 1);
    assert_eq!(player.jump_fix_frames_1308, 1);
    assert_eq!(processed.jump_fix_frames_2576, 1);
    assert_eq!(player.jump_reference_1264, output.jump_reference_128);
    assert_eq!(processed.jump_reference_848, output.jump_reference_128);
    assert_eq!(processed.flags_2468 & 0x400, 0x400);
    assert_eq!(processed.scalar_2740.to_bits(), 0x3E80_0000);
    assert_eq!(processed.flags_2480 & 0x0060_0000, 0x0060_0000);
    assert!(!candidates.first_pending_288);
    assert!(!candidates.second_pending_592);
    assert!(output.complete_76);
}

#[test]
fn reset_branch_clears_state_latch_and_staged_candidate_follows_validity() {
    let mut player = player();
    player.flags_1296 = 0x4000_0000;
    let mut processed = processed();
    processed.flags_2468 = 0x0040_0000;
    let mut output = phys_out();
    let mut candidates = candidates();
    candidates.staged_word_12768 = 0x1122_3344;
    candidates.staged_valid_12772 = 9;
    candidates.staged_pending_12780 = true;
    let mut services = Services {
        events: Vec::new(),
        trajectory: 0,
        scalar: 0.0,
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

    assert_eq!(player.state_frames_1304, 0);
    assert_eq!(processed.state_frames_2572, 0);
    assert_eq!(player.flags_1296 & 0x4000_0000, 0);
    assert_eq!(candidates.staged_latched_12776, 1);
    assert_eq!(processed.flags_2480 & 0x0010_0000, 0x0010_0000);
    assert_eq!(processed.word_2464, 0x1122_3344);
    assert!(!candidates.staged_pending_12780);
}

#[test]
fn latch_helper_preserves_the_twenty_frame_strict_expiry_and_mirrors_bits() {
    let mut player = player();
    player.flags_1296 = 0x0600_0000;
    player.latch_frames_1320 = 20;
    let mut processed = processed();

    update_flag_latches_82db5cf8(&mut player, &mut processed);
    assert_eq!(player.latch_frames_1320, 21);
    assert_eq!(player.flags_1296 & 0x0600_0000, 0x0600_0000);

    update_flag_latches_82db5cf8(&mut player, &mut processed);
    assert_eq!(player.flags_1296 & 0x0600_0000, 0);
    assert_eq!(processed.flags_2472 & 0x0000_000C, 0);

    processed.flags_2472 = 0x0000_8010;
    processed.flags_2480 = 0x0002_0000;
    update_flag_latches_82db5cf8(&mut player, &mut processed);
    assert_eq!(player.flags_1296 & 0x1802_0000, 0x1802_0000);
}
