use super::*;

const PERIOD: Duration = Duration::from_nanos(16_666_667);

fn submission() -> PresentationSubmission {
    PresentationSubmission {
        camera: CameraSubmission {
            frames: Vec::new(),
            lens: camera::CameraLens {
                near_plane: 0.2,
                far_plane: 7700.0,
                override_fov: None,
                update_counter: 0,
            },
        },
        pip_camera: None,
        skaters: Vec::new(),
        living_world: LivingWorldSubmission::default(),
        tick: 0,
        sim_rate: NORMAL_UPDATE_RATE,
    }
}

/// The PC clock's shown moment, in ticks, for frames drawn every millisecond while
/// tick `k` (due at `k` periods) reaches the presentation `latency(k)` after it is due.
fn shown(latency: impl Fn(u32) -> Duration, ticks: u32) -> Vec<f64> {
    let mut restorer = PresentationRestorer::new();
    restorer.native = false;
    let start = Instant::now();
    let due = |tick: u32| start + PERIOD * tick;
    let mut submitted = 0;
    let mut moments = Vec::new();
    let end = PERIOD * ticks;
    let mut elapsed = Duration::ZERO;
    while elapsed < end {
        let now = start + elapsed;
        while submitted < ticks && due(submitted) + latency(submitted) <= now {
            restorer.submit(
                std::sync::Arc::new(submission()),
                due(submitted),
                due(submitted) + latency(submitted),
            );
            submitted += 1;
        }
        if restorer.is_running()
            && let Some((frame, fraction)) = restorer.scheduled_moment(now)
        {
            moments.push(f64::from(frame) - 1.0 + f64::from(fraction));
        }
        elapsed += Duration::from_millis(1);
    }
    moments
}

#[test]
fn frames_between_punctual_ticks_show_evenly_spaced_moments() {
    let moments = shown(|_| Duration::from_millis(3), 30);
    let step = 1.0e-3 / PERIOD.as_secs_f64();
    let running: Vec<f64> = moments
        .into_iter()
        .skip_while(|moment| *moment <= 0.0)
        .collect();
    assert!(running.len() > 400);
    for pair in running.windows(2) {
        assert!(
            (pair[1] - pair[0] - step).abs() < 1e-4,
            "step {}",
            pair[1] - pair[0]
        );
    }
}

#[test]
fn a_late_tick_holds_the_newest_moment_and_never_goes_back() {
    let moments = shown(
        |tick| {
            if tick == 12 {
                Duration::from_millis(30)
            } else {
                Duration::from_millis(3)
            }
        },
        30,
    );
    for pair in moments.windows(2) {
        assert!(pair[1] >= pair[0], "{} then {}", pair[0], pair[1]);
    }
    assert!(moments.windows(2).any(|pair| pair[1] == pair[0]));
}

#[test]
fn a_stall_as_ticks_resume_does_not_hold_the_clock_back() {
    // The first ticks of a run arrive a stall late (an autosave), the rest punctually.
    let moments = shown(
        |tick| {
            if tick < 2 {
                Duration::from_millis(40)
            } else {
                Duration::from_millis(3)
            }
        },
        30,
    );
    let settled: Vec<f64> = moments.into_iter().filter(|moment| *moment >= 8.0).collect();
    assert!(settled.len() > 300);
    for pair in settled.windows(2) {
        assert!(pair[1] > pair[0], "{} held", pair[0]);
    }
}
