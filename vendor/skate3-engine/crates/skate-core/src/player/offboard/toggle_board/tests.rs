use super::*;

fn live(remaining: f32, elapsed: f32) -> Channel {
    Channel {
        exists: true,
        remaining,
        elapsed,
    }
}
fn retrieve() -> Input {
    Input {
        retrieve_requested: true,
        retrieval_active: true,
        ..Input::default()
    }
}

#[test]
fn retrieval_runs_into_cycle_and_out_with_pulse_only_on_cycle_entry() {
    let mut state = State::default();
    let mut input = retrieve();
    let start = state.update(input, Channel::default());
    assert_eq!(start.channel, Some(Command::BlendTo(Clip::FrontInto)));
    assert!(start.retrieving && !start.retrieve);
    assert_eq!(state.update(input, live(0.251, 0.0)).channel, None);
    let cycle = state.update(input, live(0.25, 0.0));
    assert_eq!(cycle.channel, Some(Command::SequenceTo(Clip::FrontCycle)));
    assert!(cycle.retrieve);
    assert!(!state.update(input, live(0.4, 0.0)).retrieve);
    assert!(state.update(input, live(0.25, 0.0)).retrieve);
    input.holding_board = true;
    let out = state.update(input, live(0.4, 0.0));
    assert_eq!(out.channel, Some(Command::BlendTo(Clip::FrontOut)));
    assert!(!out.retrieve);
    assert_eq!(state.update(input, live(0.4, 0.16666667)).channel, None);
    let end = state.update(input, live(0.4, 0.17));
    assert_eq!(
        end.channel,
        Some(Command::Stop {
            blend_seconds: 0.16666667
        })
    );
    assert!(end.retrieving && end.yaw_pitch.is_some());
    assert_eq!(state.phase, Phase::Idle);
    input.retrieve_requested = false;
    assert_eq!(state.update(input, Channel::default()), Output::default());
}

#[test]
fn drop_has_precedence_over_throw_and_playout_boundary_is_strict() {
    let mut state = State::default();
    let input = Input {
        holding_board: true,
        drop_requested: true,
        throw_requested: true,
        ..Input::default()
    };
    assert_eq!(
        state.update(input, Channel::default()).channel,
        Some(Command::BlendTo(Clip::Drop))
    );
    assert_eq!(state.update(input, live(1.0, 0.5)).channel, None);
    assert!(matches!(
        state.update(input, live(1.0, 0.5001)).channel,
        Some(Command::Stop { .. })
    ));
    let input = Input {
        drop_requested: false,
        ..input
    };
    assert_eq!(
        state.update(input, Channel::default()).channel,
        Some(Command::BlendTo(Clip::Throw))
    );
}

#[test]
fn physical_guards_prevent_new_requests_but_do_not_cancel_running_drop() {
    let mut state = State::default();
    assert_eq!(
        state.update(
            Input {
                retrieval_blocked: true,
                ..retrieve()
            },
            Channel::default()
        ),
        Output::default()
    );
    assert_eq!(
        state.update(
            Input {
                grabbing_object: true,
                ..retrieve()
            },
            Channel::default()
        ),
        Output::default()
    );
    assert_eq!(
        state.update(
            Input {
                holding_board: true,
                ..retrieve()
            },
            Channel::default()
        ),
        Output::default()
    );
    let drop = Input {
        holding_board: true,
        drop_requested: true,
        ..Input::default()
    };
    state.update(drop, Channel::default());
    let out = state.update(
        Input {
            grabbing_object: true,
            ..drop
        },
        live(0.6, 0.1),
    );
    assert!(out.dropping);
    assert_eq!(state.phase, Phase::DropPlaying);
}

#[test]
fn lost_retrieval_feedback_stops_before_possession_or_channel_checks() {
    let mut state = State::default();
    state.update(retrieve(), Channel::default());
    state.update(retrieve(), live(0.25, 0.0));
    let out = state.update(
        Input {
            retrieval_active: false,
            holding_board: true,
            ..retrieve()
        },
        Channel::default(),
    );
    assert!(matches!(out.channel, Some(Command::Stop { .. })));
    assert!(!out.retrieve);
}

#[test]
fn expired_into_channel_returns_idle_without_sending_stop() {
    for channel in [Channel::default(), live(0.0, 1.0)] {
        let mut state = State::default();
        state.update(retrieve(), Channel::default());
        let out = state.update(retrieve(), channel);
        assert_eq!(state.phase, Phase::Idle);
        assert_eq!(out.channel, None);
        assert!(out.retrieving);
    }
}

#[test]
fn yaw_uses_back_selection_and_five_degree_per_update_limit() {
    let mut state = State::default();
    let input = Input {
        yaw_radians: -2.0,
        pitch_radians: 0.5,
        ..retrieve()
    };
    let out = state.update(input, Channel::default());
    assert_eq!(out.channel, Some(Command::BlendTo(Clip::BackInto)));
    let first = out.yaw_pitch.unwrap();
    assert_eq!(first, [2.0 * 57.295776, 0.5 * 57.295776]);
    let next = state
        .update(
            Input {
                yaw_radians: 0.0,
                ..input
            },
            live(1.0, 0.0),
        )
        .yaw_pitch
        .unwrap();
    assert_eq!(next[0], first[0] - 5.0);
    state.begin();
    let reset = state.update(
        Input {
            yaw_radians: 0.0,
            ..input
        },
        Channel::default(),
    );
    assert_eq!(reset.channel, Some(Command::BlendTo(Clip::FrontInto)));
    assert_eq!(reset.yaw_pitch.unwrap()[0], 0.0);
}

#[test]
fn mirrored_yaw_applies_native_asymmetric_rear_sector_rule() {
    let input = Input {
        yaw_radians: 2.0,
        ..retrieve()
    };
    let mut normal = State::default();
    let out = normal.update(input, Channel::default());
    assert_eq!(out.yaw_pitch.unwrap()[0], -90.0);
    assert_eq!(out.channel, Some(Command::BlendTo(Clip::FrontInto)));
    let mut mirrored = State::default();
    let out = mirrored.update(
        Input {
            mirrored: true,
            ..input
        },
        Channel::default(),
    );
    assert_eq!(out.yaw_pitch.unwrap()[0], 2.0 * 57.295776);
    assert_eq!(out.channel, Some(Command::BlendTo(Clip::BackInto)));
    let mut rear = State::default();
    assert_eq!(
        rear.update(
            Input {
                yaw_radians: 3.0,
                ..input
            },
            Channel::default()
        )
        .yaw_pitch
        .unwrap()[0],
        180.0
    );
}
