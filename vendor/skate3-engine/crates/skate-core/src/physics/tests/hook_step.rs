use super::*;

#[test]
fn an_attached_body_exchanges_reactions_with_the_board_before_both_integrate() {
    let mut expected = bodies();
    let mut actual = expected;
    let mut target = inactive_hook();
    target.body.state_flags = 4;
    target.body.rates.position.x += 0.2;
    target.drive.enable_animation_soft(&mut 0);
    let mut attached_body = target.body;
    let config = settings(25);
    let frames = prepare_drive_frames(config.base_truck_transforms, [0.; 2], &mut target);
    let mut rows = BoardConstraints::build(
        &actual,
        &target,
        frames,
        config.truck_dynamics,
        config.simulation.time_step,
    )
    .drives;
    let mut row = rows.pop().unwrap();
    row.frame_a_body.reaction_index = ATTACHED_REACTION_BASE;
    let mut attached_drives = [row];
    let queue = BoardForceQueue::default();
    BoardStep::default().advance(&mut expected, &mut target, &queue, &[], [0.; 2], config);
    BoardStep::default().advance_attached(
        &mut actual,
        &mut inactive_hook(),
        &queue,
        &[],
        [0.; 2],
        config,
        AttachedStep {
            bodies: vec![&mut attached_body],
            contacts: &mut [],
            joints: &mut [],
            drives: &mut attached_drives,
        },
    );
    for (a, b) in actual.iter().zip(expected.iter()) {
        assert_eq!(a.rates.position, b.rates.position);
        assert_eq!(a.rates.linear_velocity, b.rates.linear_velocity);
    }
    assert_eq!(attached_body.rates.position, target.body.rates.position);
    assert_eq!(
        attached_body.rates.linear_velocity,
        target.body.rates.linear_velocity
    );
    assert!(
        attached_drives[0]
            .accumulated_linear_impulse
            .iter()
            .any(|v| *v != 0.)
    );
}

#[test]
fn hook_drive_exchanges_momentum_and_integrates_without_teleporting_deck() {
    let mut parts = bodies();
    let mut hook = inactive_hook();
    hook.body.state_flags = 4;
    hook.body.rates.position.x += 0.2;
    let mut animated = 0;
    hook.drive.enable_animation_soft(&mut animated);
    let mut disabled = hook.clone();
    disabled.drive.disable_animation(&mut animated);
    let mut free = parts;
    let original_deck = parts[6].rates.position;
    let queue = BoardForceQueue::default();
    let mut tick = BoardStep::default();
    let config = settings(25);
    tick.advance(&mut parts, &mut hook, &queue, &[], [0.0; 2], config);
    BoardStep::default().advance(&mut free, &mut disabled, &queue, &[], [0.0; 2], config);
    assert_ne!(parts[6].rates.position.x, free[6].rates.position.x);
    assert_ne!(hook.body.rates.position.x, disabled.body.rates.position.x);
    assert_ne!(parts[6].rates.position.x, hook.body.rates.position.x);
    assert_ne!(parts[6].rates.position, original_deck);
    let momentum = parts
        .iter()
        .chain(core::iter::once(&hook.body))
        .map(|b| b.rates.linear_velocity.x / b.inertia.inverse_mass)
        .sum::<f32>();
    assert!(
        momentum.abs() < 1e-4,
        "internal hook drive changed momentum: {momentum}"
    );
    assert_eq!(
        tick.reactions,
        [RetailReactionCorrections::default(); REACTION_COUNT]
    );
}
