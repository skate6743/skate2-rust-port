//! Controlled first skating view over the retained core session.
//! The selected spawn location's original map meshes are preloaded through the
//! inspection backend.
//! Spawn scheduling and restored camera presentation remain pending.
use crate::{
    gpu_renderer::{GpuRenderer, GpuScene, RenderResult},
    juice_platform::{PcJuiceConnection, PcJuicePlatform},
    live_skating_view::LiveSkatingView,
    proxy_world::{ProxyHighCollections, ProxyWorldOwner},
    render_inspection::Inspection,
    render_window::{ClientFrameTimings, RenderWindowClient, RenderWindowOptions},
    sdl_pads::SdlPads,
};
#[cfg(not(windows))]
use crate::sdl_pad_platform::SdlPadPlatform as Pads;
#[cfg(windows)]
use crate::windows_pad_platform::WindowsPadPlatform as Pads;
use skate2_core::{
    JuiceHostOption, SelectedBootProgress, SelectedCommandStartPolicy,
    SelectedControlledBootPolicy, SelectedFrameNumerics, SelectedGestureAllocation,
    SelectedGravityPolicy, SelectedLiveSession, SelectedQueuedFrameOutcome,
    SelectedQueuedFramePolicy, SelectedRootWPolicy, SelectedSpawnPolicy, XboxPadSample,
    collection_resident_streamer::CollectionId,
    game::{CommonBackEndSeed, WorldStreamSpec},
    player::{DEFAULT_MARQUEE_ID, selected_spawn_location},
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    world_stream_catalog::WorldStreamCatalog,
};
use skate2_formats::{
    location::LocationTable,
    mount::MountSet,
    stream::{CollectionMap, high_detail_collection_filename},
};
use skate2_core::presentation_restore::PresentationRestorer;
use std::{
    convert::Infallible,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use winit::event::WindowEvent;

const FIXED_STEP: Duration = Duration::from_nanos(16_666_667);
#[path="replay_entry_probe.rs"]
mod replay_entry_probe;
const SIM_LOCATIONS_PATH: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";
const SIM_COLLECTION_MAP_PATH: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.xsm";

/// `cAssetStreamSystem::GetCollectionId` 0x824d73b0: the high-detail Sim
/// collection under the spawn location, resolved by name in `BAM_Sim.xsm`.
fn spawn_sim_collection(mounts: &MountSet) -> RenderResult<CollectionId> {
    let locations = LocationTable::parse(&mounts.read_path(SIM_LOCATIONS_PATH)?)?;
    let position = locations
        .find(selected_spawn_location())
        .ok_or("spawn location absent from BAM_Sim.loc")?
        .position();
    let filename = high_detail_collection_filename("Sim", position);
    let map = CollectionMap::parse(&mounts.read_path(SIM_COLLECTION_MAP_PATH)?)?;
    map.collections
        .iter()
        .find(|entry| entry.filename == filename)
        .map(|entry| CollectionId(entry.id))
        .ok_or_else(|| format!("{filename} absent from BAM_Sim.xsm").into())
}
const MAX_TICKS_PER_REDRAW: u32 = 4;

// Keep one timer period of overlap for SIM and consume older debt within the
// existing redraw budget. This never changes dt or discards an elapsed slot.
fn catch_up_due(accumulator: Duration, step: Duration, ticks: u32) -> bool {
    ticks < MAX_TICKS_PER_REDRAW && accumulator >= step.saturating_mul(2)
}

/// Host scheduling only: choose the smallest catch-up batch that can sustain
/// the timer rate under the measured pipeline costs. One SIM tick overlaps
/// presentation; extra ticks serialize with their completion handshakes.
/// If the workload cannot sustain the timer rate, retain the batch with the
/// best predicted tick throughput. No timer slots or tick work are discarded.
fn pipeline_tick_budget(step: Duration, tick: Duration, completion: Duration,
    finish: Duration, presentation: Duration) -> u32
{
    // When SIM already covers presentation, another serialized tick cannot
    // hide any drawing cost. Amortizing the tiny once-per-draw finish cost
    // used to select four even here, turning a small tick overrun into a
    // four-tick wait before every image. Give the next tick the same render
    // overlap instead. All debt remains due; GPU-bound frames still batch.
    if tick >= presentation {
        return 1;
    }
    let period = |n: u32| tick.max(presentation)
        .saturating_add(tick.saturating_mul(n - 1))
        .saturating_add(completion.saturating_mul(n))
        .saturating_add(finish);
    let mut best = 1;
    for n in 1..=MAX_TICKS_PER_REDRAW {
        let cost = period(n);
        if cost <= step.saturating_mul(n) {
            return n;
        }
        if cost.as_nanos() * u128::from(best) < period(best).as_nanos() * u128::from(n) {
            best = n;
        }
    }
    best
}


/// One more tick is due while the debt covers the armed timer period and the
/// per-redraw budget remains.
fn tick_due(accumulator: Duration, step: Duration, ticks: u32) -> bool {
    ticks < MAX_TICKS_PER_REDRAW && accumulator >= step
}

/// `SetTimerHz`82985ec0: TimerPeriod = 10,000,000 / Hz in 100 ns units. The
/// 60 Hz baseline keeps the host's exact 1/60 s step.
fn timer_period(hz: i32) -> Duration {
    // SKATE2_DIAGNOSTIC_TIMER_HZ=<hz>: the 60 Hz baseline fires at that rate instead, as on a
    // machine too slow to keep up (online pacing tests).
    static DIAGNOSTIC_HZ: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
    let diagnostic = *DIAGNOSTIC_HZ.get_or_init(|| std::env::var("SKATE2_DIAGNOSTIC_TIMER_HZ").ok().and_then(|hz| hz.trim().parse::<i32>().ok()).filter(|&hz| hz > 0));
    let hz = if hz == 60 { diagnostic.unwrap_or(hz) } else { hz };
    if hz == 60 {
        FIXED_STEP
    } else {
        let hz = u64::try_from(hz).expect("the Simulation refuses non-positive timer rates");
        Duration::from_nanos(10_000_000 / hz * 100)
    }
}

type PcSession = SelectedLiveSession<fn() -> Result<u64, Infallible>, PcJuiceConnection>;

/// A process-local host identity mapping. Its value is not an Xbox thread ID.
fn host_animation_thread() -> Result<u64, Infallible> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    Ok(hasher.finish())
}

/// The game, from the original game's folder `root`.
pub(crate) fn run(root: &std::path::Path) -> RenderResult<()> {
    let _lean_capture = skate2_core::diagnostics::lean_trace::SaveOnExit;
    let args: Vec<_> = std::env::args().collect();
    let option = |name: &str| -> RenderResult<Option<&str>> {
        args.iter()
            .position(|argument| argument == name)
            .map(|index| {
                args.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("{name} requires a value").into())
            })
            .transpose()
    };
    let frame_limit = option("--live-frames")?.map(str::parse).transpose()?;
    let completion_limit: Option<u64> =
        option("--live-completions")?.map(str::parse).transpose()?;
    let capture = option("--live-capture")?.map(PathBuf::from);
    let ollie_demo = args.iter().any(|argument| argument == "--live-ollie-demo");
    let ollie_pattern = option("--live-ollie-pattern")?.unwrap_or("Ollie").to_owned();
    let ollie_hold: u64 = option("--live-ollie-hold")?
        .map(str::parse)
        .transpose()?
        .unwrap_or(2);
    let ollie_period: Option<u64> = option("--live-ollie-period")?.map(str::parse).transpose()?;
    if ollie_period.is_some_and(|period| period < ollie_hold + 120) {
        return Err("--live-ollie-period must leave at least120 ticks after the crouch".into());
    }
    let ollie_spin_left = args
        .iter()
        .any(|argument| argument == "--live-ollie-spin-left");
    let ollie_spin = ollie_spin_left || args.iter().any(|argument| argument == "--live-ollie-spin");
    let controls_demo = match option("--live-controls-demo")? {
        None => None,
        Some("offboard-jump") => Some(ControlsDemo::OffboardJump),
        Some("offboard-city-walk") => Some(ControlsDemo::OffboardCityWalk),
        Some("pedestrian-approach") => Some(ControlsDemo::PedestrianApproach),
        Some("offboard-city-exit") => Some(ControlsDemo::OffboardCityExit),
        Some("push-bottom-left") => Some(ControlsDemo::PushBottomLeft),
        Some("bail-respawn") => Some(ControlsDemo::BailRespawn),
        Some("bail-auto") => Some(ControlsDemo::BailAuto),
        Some("bail-gestures") => Some(ControlsDemo::BailGestures),
        Some("offboard-right-left") => Some(ControlsDemo::OffboardRightLeft),
        Some("offboard-right-right") => Some(ControlsDemo::OffboardRightRight),
        Some("marker-ui") => Some(ControlsDemo::MarkerUi),
        Some("edit-skater") => Some(ControlsDemo::EditSkater),
        Some("main-menu") => Some(ControlsDemo::MainMenu),
        Some("main-menu-roundtrip") => Some(ControlsDemo::MainMenuRoundtrip),
        Some("main-menu-gameplay") => Some(ControlsDemo::MainMenuGameplay),
        Some("main-menu-freeskate") => Some(ControlsDemo::MainMenuFreeskate),
        Some("career") => Some(ControlsDemo::Career),
        Some("display-settings") => Some(ControlsDemo::DisplaySettings),
        Some("downhill-manual") => Some(ControlsDemo::DownhillManual),
        Some("edit-skater-roundtrip") => Some(ControlsDemo::EditSkaterRoundtrip),
        Some("edit-skater-couch") => Some(ControlsDemo::EditSkaterCouch),
        Some("edit-skater-styles") => Some(ControlsDemo::EditSkaterStyles),
        Some("edit-skater-eyes") => Some(ControlsDemo::EditSkaterEyes),
        Some("edit-skater-gender") => Some(ControlsDemo::EditSkaterGender),
        Some("edit-skater-popup") => Some(ControlsDemo::EditSkaterPopup),
        Some("edit-skater-remove-shirt") => Some(ControlsDemo::EditSkaterRemoveShirt),
        Some("map-teleport") => Some(ControlsDemo::MapTeleport),
        Some("marker-return") => Some(ControlsDemo::MarkerReturn),
        Some("offboard-street-look") => Some(ControlsDemo::OffboardStreetLook),
        Some("postfx-sun-look") => Some(ControlsDemo::PostfxSunLook),
        Some("perf-skate") => Some(ControlsDemo::PerfSkate),
        Some("hippy-spin") => Some(ControlsDemo::HippySpin),
        Some(_) => return Err("unknown --live-controls-demo sequence".into()),
    };
    let ollie_tweak_left = args
        .iter()
        .any(|argument| argument == "--live-ollie-tweak-left");
    let ground_grab = match option("--live-ground-grab")? {
        None => None,
        Some("left") => Some(1_u8),
        Some("right") => Some(2_u8),
        Some("both") => Some(3_u8),
        Some(_) => return Err("--live-ground-grab requires left, right or both".into()),
    };
    let manual_demo = args.iter().any(|argument| argument == "--live-manual-demo");
    if !(1..=600).contains(&ollie_hold) {
        return Err("--live-ollie-hold must be between 1 and 600 ticks".into());
    }
    let y_demo = args.iter().any(|argument| argument == "--live-y-demo");
    let push_demo = args.iter().any(|argument| argument == "--live-push-demo");
    let trace_selected_states =
        std::env::var("SKATE2_TRACE_SELECTED_STATES").is_ok_and(|value| value == "1");
    let hold_at_completion = args.iter().any(|argument| argument == "--live-hold");
    if [
        ollie_demo,
        y_demo,
        push_demo,
        ground_grab.is_some(),
        manual_demo,
        controls_demo.is_some(),
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        return Err("choose only one live input demo".into());
    }
    if frame_limit.is_some() && completion_limit.is_some() {
        return Err("choose --live-frames or --live-completions, not both".into());
    }
    if completion_limit == Some(0) {
        return Err("completion limit must be positive".into());
    }
    if capture.is_some() && frame_limit.is_none() && completion_limit.is_none() {
        return Err("--live-capture requires --live-frames or --live-completions".into());
    }
    if hold_at_completion && completion_limit.is_none() {
        return Err("--live-hold requires --live-completions".into());
    }
    let mounts = Arc::new(crate::mount_reference_archives(root)?);
    let terrain_catalog = WorldStreamCatalog::new();
    // `SyncLoadProxyWorldStream`'s order and proxy map precede the world
    // stream, whose `AddHighCollection` calls fill the high-to-proxy map.
    let proxy_world = ProxyWorldOwner::load_shared(&mounts)?;
    let mut map = Inspection::load(&mounts)?;
    map.enable_city_streaming(
        Arc::clone(&mounts),
        terrain_catalog.clone(),
        Arc::clone(&proxy_world),
    );
    // These sequences start neutral. The older demo seed presses D-pad Up
    // and can start a gesture before the scripted Y dismount is sampled.
    let input_fuzz = crate::input_fuzz::InputFuzz::from_env();
    let demo_sample = if controls_demo.is_some() {
        Some(demo_y_sample(false))
    } else if input_fuzz.is_some() {
        Some(neutral_demo_sample())
    } else if ollie_demo || push_demo || ground_grab.is_some() || manual_demo {
        Some(neutral_demo_sample())
    } else {
        y_demo.then(|| demo_y_sample(false))
    };
    // A scripted replay starts no SDL pads: on Windows it reads XInput's slots only, as it
    // always has.
    let sdl = demo_sample.is_none().then(SdlPads::start);
    let mut pad = Pads::new(demo_sample, sdl);
    let mut juice = PcJuicePlatform::new(None);
    // A retail game folder has no data\juice: Juice keeps its defaults, and starts disabled here
    // either way.
    let juice_ini = skate2_formats::host_path::read_if_present(&root.join("data").join("juice").join("Juice_m.ini"))?;
    // The retained native-owner graph is large. Keep it at one stable heap
    // address before moving the window client into winit's event callback.
    let mut session = Box::new(PcSession::new(Arc::clone(&mounts), root));
    session.set_career_calendar(crate::local_calendar::now);
    let first_test = SelectedControlledBootPolicy {
        spawn_policy: SelectedSpawnPolicy::AuthoredSelectedLocation,
        marquee: DEFAULT_MARQUEE_ID,
        actor_flags: 1,
        capsule_resource_allocated: true,
        camera_set: CommonBackEndSeed::debug_reference().camera_set.to_owned(),
        terrain_catalog,
        terrain_spec: WorldStreamSpec::derive("BAM", None)?,
        unlock_seed: EnvironmentUnlockSeed {
            locations_lock_status_raw: 0,
            contexts_in_native_iteration_order: Vec::new(),
        },
        collection: spawn_sim_collection(&mounts)?,
        dmo_update_in_parallel: false,
        arena_userdata: 0,
        presentation_lod: 0,
        presentation_submission_capacity: 65536,
        presentation_gather_capacity: 65536,
        presentation_provider: 0,
        // This still transports an opaque source W lane as a first-test
        // policy; it is not a recovered native skater-W producer.
        presentation_root_w: SelectedRootWPolicy::ReadSkaterOpaque { bits: 0 },
        camera_submit_mask: 3,
        gravity_policy: SelectedGravityPolicy::VaultSkateboardDefault,
        command_start: SelectedCommandStartPolicy::NativeStaticFields,
    };
    let progress = session.try_begin_controlled(
        &mut pad,
        &mut juice,
        juice_ini.as_deref(),
        JuiceHostOption::DisableBeforeStart,
        host_animation_thread as fn() -> Result<u64, Infallible>,
        ProxyHighCollections(proxy_world),
        first_test,
    )?;
    let spawn_location = selected_spawn_location();
    eprintln!(
        "CONTROLLED FIRST TEST: boot={progress:?}; original static {spawn_location} map, diagnostic materials, provisional CameraMan projection, native spawn/status pending"
    );
    let completed_counter = Arc::new(AtomicU64::new(0));
    // Existing bounded/scripted physics diagnostics keep their explicit direct
    // gameplay entry. Ordinary controller play starts in the native frontend.
    if (controls_demo.is_none() && !ollie_demo && completion_limit.is_none())
        || matches!(controls_demo,Some(ControlsDemo::MainMenu|ControlsDemo::MainMenuRoundtrip|ControlsDemo::MainMenuGameplay|ControlsDemo::MainMenuFreeskate|ControlsDemo::DisplaySettings|ControlsDemo::Career)) {
        session.start_main_menu(&mounts)?;
        if let Some(changelog) = crate::game_update::changelog() {
            session.show_changelog(changelog);
        }
    }
    let presented_completion_counter = Arc::new(AtomicU64::new(0));
    let client = LiveSkatingClient {
        tick_trace: TickTrace::from_environment(),
        frame_timings: ClientFrameTimings::default(),
        session,
        pad,
        ready: false,
        gameplay: false,
        controlled_run_start: None,
        last_progress: Some(progress),
        previous: Instant::now(),
        accumulator: Duration::ZERO,
        step: FIXED_STEP,
        backlogged_redraws: 0,
        completed_frames: 0,
        completion_limit,
        completed_counter: Arc::clone(&completed_counter),
        presented_completion_counter: Arc::clone(&presented_completion_counter),
        no_command_reported: false,
        // Wait for the window's actual focus event before admitting focus-gain input.
        focused: false,
        ollie_demo,
        ollie_pattern,
        ollie_hold,
        ollie_period,
        ollie_spin,
        ollie_spin_left,
        controls_demo,
        input_fuzz,
        frontend_demo_ticks: 0,
        replay_probe:std::env::var_os("SKATE2_DIAGNOSTIC_REPLAY_ENTRY").map(|_|replay_entry_probe::Probe::default()),
        update: crate::game_update::GameUpdate::start(),
        trick_guide_previous:None,
        pedestrian_approach_stick: [0, i16::MAX],
        ollie_tweak_left,
        ground_grab,
        manual_demo,
        y_demo,
        push_demo,
        push_start_position: None,
        trace_selected_states,
        ollie_points: None,
        freeskate_location_entered: false,
        party_location_started: false,
        party_dialog_seen: false,
        party_first_player_turns: 0,
        party_change_step: 0,
        party_change_wait: 0,
        party_quit_menu_seen: false,
    };
    crate::render_window::run_with_options(
        LiveSkatingWindow::new(client, LiveSkatingView::new(map, Arc::clone(&mounts)))?,
        RenderWindowOptions {
            frame_limit,
            capture,
            title: Some(if ollie_demo {
                "Skate 2 - scripted original Ollie input / Esc to close"
            } else if y_demo {
                "Skate 2 - scripted raw Y OffBoard recovery test / Esc to close"
            } else if push_demo {
                "Skate 2 - scripted raw A push / Esc to close"
            } else {
                "sk82"
            }),
            hold_at_completion,
        },
    )?;
    let completed = completed_counter.load(Ordering::Relaxed);
    // These diagnostics complete on the frontend clock while SIM is paused;
    // their menu/roundtrip assertions run in frame(), not on the SIM counter.
    if matches!(controls_demo,Some(ControlsDemo::MainMenu|ControlsDemo::MainMenuRoundtrip|ControlsDemo::MainMenuGameplay|ControlsDemo::MainMenuFreeskate|ControlsDemo::DisplaySettings|ControlsDemo::Career)) {
        return Ok(());
    }
    if frame_limit.is_some() && completed == 0 {
        return Err("bounded live test presented no completed selected skating frame".into());
    }
    if let Some(limit) = completion_limit {
        let presented_completion = presented_completion_counter.load(Ordering::Relaxed);
        if presented_completion < limit {
            return Err(format!(
                "bounded live test prepared {completed} and presented {presented_completion} of {limit} completed selected skating frames"
            )
            .into());
        }
    }
    Ok(())
}

struct LiveSkatingClient {
    tick_trace: Option<TickTrace>,
    frame_timings: ClientFrameTimings,
    session: Box<PcSession>,
    pad: Pads,
    ready: bool,
    /// The last tick completed gameplay rather than a frontend-only tick.
    gameplay: bool,
    controlled_run_start: Option<Instant>,
    last_progress: Option<SelectedBootProgress>,
    previous: Instant,
    accumulator: Duration,
    /// Period of the currently armed ExecutionMan timer.
    step: Duration,
    backlogged_redraws: u64,
    completed_frames: u64,
    completion_limit: Option<u64>,
    completed_counter: Arc<AtomicU64>,
    presented_completion_counter: Arc<AtomicU64>,
    no_command_reported: bool,
    focused: bool,
    ollie_demo: bool,
    ollie_pattern: String,
    ollie_spin_left: bool,
    controls_demo: Option<ControlsDemo>,
    input_fuzz: Option<crate::input_fuzz::InputFuzz>,
    frontend_demo_ticks: u64,
    replay_probe:Option<replay_entry_probe::Probe>,
    /// PC: a release build's updater (`game_update`).
    update: Option<crate::game_update::GameUpdate>,
    trick_guide_previous:Option<Instant>,
    pedestrian_approach_stick: [i16; 2],
    ollie_hold: u64,
    ollie_period: Option<u64>,
    ollie_spin: bool,
    ollie_tweak_left: bool,
    ground_grab: Option<u8>,
    manual_demo: bool,
    y_demo: bool,
    push_demo: bool,
    push_start_position: Option<[f32; 3]>,
    trace_selected_states: bool,
    ollie_points: Option<[[f32; 2]; 2]>,
    /// The Free Skate location check (`SKATE2_DIAGNOSTIC_FREESKATE_LOCATION`): whether its
    /// location was entered.
    freeskate_location_entered: bool,
    /// The Party Play diagnostic: whether its location was started, whether the press-A
    /// dialog showed since the last turn began, and how many of the first player's turns have
    /// begun.
    party_location_started: bool,
    party_dialog_seen: bool,
    party_first_player_turns: u32,
    /// The Change Game check's next press, and the ticks its screen has been ready.
    party_change_step: u32,
    party_change_wait: u32,
    /// The Party Play quit check reached the main menu.
    party_quit_menu_seen: bool,
}

fn neutral_demo_sample() -> XboxPadSample {
    XboxPadSample {
        buttons: 1,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    }
}

#[derive(Clone, Copy)]
enum ControlsDemo {
    MainMenu,
    MainMenuRoundtrip,
    MainMenuGameplay,
    /// Main menu row 2: FreeSkateOption's next skater, two locations on, two steps less
    /// traffic, then A.
    MainMenuFreeskate,
    /// Main menu row 0 into the career; Start closes a New Career editor.
    Career,
    DisplaySettings,
    DownhillManual,
    EditSkater,
    EditSkaterRoundtrip,
    EditSkaterCouch,
    EditSkaterStyles,
    EditSkaterEyes,
    EditSkaterGender,
    EditSkaterPopup,
    EditSkaterRemoveShirt,
    MapTeleport,
    MarkerUi,
    MarkerReturn,
    OffboardJump,
    OffboardCityWalk,
    PedestrianApproach,
    OffboardCityExit,
    PushBottomLeft,
    BailRespawn,
    BailAuto,
    BailGestures,
    OffboardRightLeft,
    OffboardRightRight,
    OffboardStreetLook,
    PostfxSunLook,
    PerfSkate,
    HippySpin,
}

impl ControlsDemo {
    /// Raw XInput sequences only: no forced graph/physics state or recovery.
    fn sample(self, frame: u64) -> XboxPadSample {
        let mut sample = demo_y_sample(false);
        match self {
            Self::DownhillManual => {
                if (30..100).contains(&frame) { sample.buttons=0x1000; }
                if frame>=110 {
                    let phase=(frame-110)%180;
                    sample.right_y=if phase<35{i16::MIN}else if (36..38).contains(&phase){i16::MAX}
                        else if phase>=48{-18000}else{0};
                }
            }
            Self::EditSkater | Self::EditSkaterEyes | Self::EditSkaterRoundtrip | Self::EditSkaterCouch | Self::EditSkaterStyles
            | Self::EditSkaterGender | Self::EditSkaterPopup | Self::EditSkaterRemoveShirt
            | Self::MapTeleport | Self::MainMenu | Self::MainMenuRoundtrip | Self::MainMenuGameplay | Self::MainMenuFreeskate | Self::DisplaySettings | Self::Career => {} // Menu-aware raw button pulses below.
            Self::MarkerReturn => {
                if (60..180).contains(&frame) {sample.buttons=0x0100;}
                if frame==150 {sample.buttons|=0x0002;}
                if (200..360).contains(&frame) {sample.buttons=0x1000;}
                if (420..540).contains(&frame) {sample.buttons=0x0101;}
            }
            Self::MarkerUi => {
                if (60..240).contains(&frame) || (300..540).contains(&frame) {
                    sample.buttons = 0x0100;
                }
                if frame == 150 || frame == 390 {
                    sample.buttons |= 0x0002;
                }
                if (460..500).contains(&frame) {
                    sample.buttons |= 0x0001;
                }
            }
            Self::PerfSkate => {
                // Benchmark-only raw pad: hold A to push off, then push for one
                // second every four. No steering or tricks; physics stays live.
                if (30..300).contains(&frame) || (frame >= 300 && (frame - 300) % 240 < 60) {
                    sample.buttons = 0x1000;
                }
            }
            Self::HippySpin => {
                // Pushes off, then every five seconds holds both push buttons for a hippy jump.
                // SKATE2_HIPPY_STICK is the stick's X (default full right), SKATE2_HIPPY_WIND_UP
                // how many frames the buttons are held (default 40) and SKATE2_HIPPY_SPIN the
                // frames of each jump the stick is held, as start..end (default 40..100).
                // A push between jumps keeps the speed up.
                let env = |name: &str| std::env::var(name).ok();
                let stick = env("SKATE2_HIPPY_STICK")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(i16::MAX);
                let wind_up = env("SKATE2_HIPPY_WIND_UP")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(40);
                let spin = env("SKATE2_HIPPY_SPIN")
                    .and_then(|value| {
                        let (start, end) = value.split_once("..")?;
                        Some(start.parse().ok()?..end.parse().ok()?)
                    })
                    .unwrap_or(40..100);
                if (30..150).contains(&frame) {
                    sample.buttons = 0x1000;
                }
                if frame >= 240 {
                    let phase = (frame - 240) % 300;
                    if phase < wind_up {
                        sample.buttons = 0x5000;
                    }
                    if spin.contains(&phase) {
                        sample.left_x = stick;
                    }
                    if (220..260).contains(&phase) {
                        sample.buttons = 0x1000;
                    }
                }
            }
            Self::PostfxSunLook => {
                if frame == 32 { sample.buttons = 0x8000; }
                if (160..760).contains(&frame) {
                    sample.left_x = -23170;
                    sample.left_y = 23170;
                }
                if (800..860).contains(&frame) { sample.right_x = 18000; }
                if (860..910).contains(&frame) { sample.right_y = -18000; }
            }
            Self::OffboardStreetLook => {
                if frame == 32 {
                    sample.buttons = 0x8000;
                }
                if (160..3400).contains(&frame) {
                    sample.right_x = if frame < 1780 { 20000 } else { -20000 };
                }
            }
            Self::OffboardCityExit => {
                if frame == 32 {
                    sample.buttons = 0x8000;
                }
                if (160..6000).contains(&frame) {
                    sample.left_y = i16::MIN;
                }
            }
            Self::OffboardCityWalk | Self::PedestrianApproach => {
                if frame == 32 {
                    sample.buttons = 0x8000;
                }
                if (160..3600).contains(&frame) {
                    sample.left_y = i16::MAX;
                }
            }
            Self::OffboardJump | Self::OffboardRightLeft | Self::OffboardRightRight => {
                if frame == 32 {
                    sample.buttons = 0x8000;
                }
                if (160..300).contains(&frame) {
                    match self {
                        Self::OffboardJump => sample.left_y = i16::MAX,
                        Self::OffboardRightLeft => sample.right_x = i16::MIN,
                        Self::OffboardRightRight => sample.right_x = i16::MAX,
                        _ => unreachable!(),
                    }
                }
                if matches!(self, Self::OffboardJump) && (220..230).contains(&frame) {
                    sample.buttons |= 0x4000; // FaceX: offboard jump.
                }
            }
            Self::PushBottomLeft => {
                if (32..140).contains(&frame) {
                    sample.buttons = 0x1000;
                }
                if (160..300).contains(&frame) {
                    sample.left_x = -23170;
                    sample.left_y = -23170;
                }
            }
            Self::BailRespawn | Self::BailAuto | Self::BailGestures => {
                if frame == 32 {
                    sample.buttons = 0x00c0; // Both stick clicks + both triggers.
                    sample.left_trigger = 255;
                    sample.right_trigger = 255;
                }
                if matches!(self, Self::BailRespawn) && frame == 160 {
                    sample.buttons = 0x1000; // Newly pressed A: WipeOutRecover.
                }
                if matches!(self, Self::BailGestures) && (40..168).contains(&frame) {
                    (sample.right_x, sample.right_y) = match (frame - 40) / 32 {
                        0 => (0, i16::MAX),
                        1 => (i16::MAX, 0),
                        2 => (0, i16::MIN),
                        _ => (i16::MIN, 0),
                    };
                }
            }
        }
        sample
    }
}

/// SKATE2_DIAGNOSTIC_SUBTITLE_SHOT: the front-end lines counted so far, the last one, and when the
/// wanted one came up.
static SUBTITLE_SHOT: std::sync::Mutex<(u32, Option<u32>, Option<Instant>)> = std::sync::Mutex::new((0, None, None));

fn subtitle_shot(wanted: u32, line: Option<u32>, shown: Option<u32>, subtitles: bool) {
    let Ok(mut shot) = SUBTITLE_SHOT.lock() else { return };
    let (count, last, at) = &mut *shot;
    if line != *last {
        *last = line;
        if line.is_some() {
            *count += 1;
            eprintln!("diagnostic: subtitle line {count} {line:x?}, shown {shown:x?}, option {subtitles}");
            if *count == wanted {
                *at = Some(Instant::now());
            }
        }
    }
}

fn subtitle_shot_done() -> bool {
    let delay = std::env::var("SKATE2_DIAGNOSTIC_SUBTITLE_SHOT").ok()
        .and_then(|shot| shot.split(',').nth(1)?.parse::<u64>().ok()).unwrap_or(500);
    SUBTITLE_SHOT.lock().is_ok_and(|shot| shot.2.is_some_and(|at| at.elapsed() >= Duration::from_millis(delay)))
}

/// SKATE2_DIAGNOSTIC_BUY_PROPERTY's phase: 0 idle, 1 buying, 2 backing out after a failure.
static BUY_PHASE: AtomicU64 = AtomicU64::new(0);

/// SKATE2_DIAGNOSTIC_BUY_PROPERTY's input: Start opens the hub, A its Map row, then on the
/// career map down to the Property category, A, A on its first item and A on the info
/// component's option; on the popup A (OK) for a failure, down to Yes and A for the
/// confirmation. After a failure B backs out of the info component, the category and the map,
/// then Start closes the hub.
fn buy_property_step(map: &skate2_core::ui::map_menu::MapMenuSnapshot, pause: skate2_core::ui::pause_menu::PauseMenuSnapshot) -> Option<u16> {
    use skate2_core::challenge::stuff_menu::{MapPopup, MenuText};
    let (up, down, start, a, b) = (0x0001, 0x0002, 0x0010, 0x1000, 0x2000);
    let phase = BUY_PHASE.load(Ordering::Relaxed);
    if phase == 0 {
        return None;
    }
    let property = |text: &MenuText| matches!(text, MenuText::Category { hal: "ID_STUFFMENU_CATEGORY_PROPERTY", .. });
    if map.visible {
        let view = map.career.as_deref()?;
        if let Some((popup, selected)) = view.popup {
            eprintln!("diagnostic: buy property popup {popup:?}, option {selected}");
            return Some(match popup {
                MapPopup::PurchaseFailed => {
                    BUY_PHASE.store(2, Ordering::Relaxed);
                    a
                }
                MapPopup::PurchaseConfirm if selected == 0 => down,
                MapPopup::PurchaseConfirm => {
                    BUY_PHASE.store(0, Ordering::Relaxed);
                    a
                }
                MapPopup::DismissBigBlack => return None,
            });
        }
        if phase == 2 {
            return Some(b);
        }
        if let Some(info) = &view.info {
            if let Some(property) = &info.property {
                eprintln!("diagnostic: buy property info {:?} option {} price {:?} cash {:?}", info.title, info.option, property.price, property.cash);
            }
            return Some(if info.property.is_some() { a } else { b });
        }
        if property(&view.title) {
            return Some(if view.selected == 0 { a } else { up });
        }
        let Some(row) = view.rows.iter().position(property) else {
            eprintln!("diagnostic: buy property: no Property row in {:?}", view.rows);
            return None;
        };
        return Some(match view.selected.cmp(&row) {
            std::cmp::Ordering::Less => down,
            std::cmp::Ordering::Greater => up,
            std::cmp::Ordering::Equal => a,
        });
    }
    if pause.visible {
        if phase == 2 {
            BUY_PHASE.store(0, Ordering::Relaxed);
            return Some(start);
        }
        return Some(if pause.options()[pause.selected] == skate2_core::ui::pause_menu::PauseMenuOption::Map { a } else { down });
    }
    (phase == 1).then_some(start)
}

/// Match the raw-pad Y corpus: FaceY is XInput bit 15 for one selected tick.
static SHOP_PHASE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static SHOP_THUMBS: AtomicBool = AtomicBool::new(false);

/// A newly opened editor shops from the start.
fn shop_session(open: bool) {
    static OPEN: AtomicBool = AtomicBool::new(false);
    if open && !OPEN.swap(true, Ordering::Relaxed) {
        SHOP_PHASE.store(0, Ordering::Relaxed);
    }
    if !open {
        OPEN.store(false, Ordering::Relaxed);
    }
}

/// SKATE2_DIAGNOSTIC_SHOP's input: Start at Back Alley Pete's (or Slappy's Merch from the Edit
/// Skater menu), A on the Slappy's row, then the first part for sale in it, B back to Slappy's
/// and B again for the receipt, A on Purchase, then Start until the editor closes. With
/// SKATE2_DIAGNOSTIC_SHOP_EDIT_CART the first receipt goes through Edit Cart first: X takes the
/// line off and on again, B returns to the picker. With SKATE2_DIAGNOSTIC_SHOP_SPREE the first
/// thumbnails take every part for sale until the balance runs out: A on the popup opens the
/// cart, whose B keeps it.
fn shop_step(view: &skate2_core::ui::edit_skater::EditSkaterView, row: usize) -> u16 {
    use skate2_core::ui::edit_skater::EditSkaterMenu as M;
    let (up, down, left, right, start, a, b) = (0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x1000, 0x2000);
    if view.menu != M::PartThumbnails {
        SHOP_THUMBS.store(false, Ordering::Relaxed);
    }
    if let Some(receipt) = &view.receipt {
        let lines: Vec<_> = receipt.lines.iter().map(|line| (line.component.as_str(), line.price, line.kept)).collect();
        eprintln!("diagnostic: shop receipt cash {} subtotal {} balance {} lines {lines:?}", receipt.cash, receipt.subtotal, receipt.balance);
        SHOP_PHASE.store(2, Ordering::Relaxed);
        static CART: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
        if std::env::var_os("SKATE2_DIAGNOSTIC_SHOP_EDIT_CART").is_some() {
            let x = 0x4000;
            match CART.fetch_add(1, Ordering::Relaxed) {
                0 => return down,
                1 => return a,
                2 | 3 => return x,
                4 => return b,
                _ => {}
            }
        }
        return if receipt.list { b } else if receipt.option > 0 { up } else { a };
    }
    static SPREE_DONE: AtomicBool = AtomicBool::new(false);
    if view.cash_popup {
        eprintln!("diagnostic: shop not enough cash (wallet {})", view.wallet);
        SPREE_DONE.store(true, Ordering::Relaxed);
        return a;
    }
    if view.exit_confirmation.is_some() {
        return a;
    }
    let spree_mode = std::env::var_os("SKATE2_DIAGNOSTIC_SHOP_SPREE").is_some();
    let spree = spree_mode && !SPREE_DONE.load(Ordering::Relaxed);
    match (SHOP_PHASE.load(Ordering::Relaxed), view.menu) {
        (0, M::BackAlleyPete) => start,
        (0, M::EditSkaterMain) if view.selected < 1 => down,
        (0, M::EditSkaterMain) => a,
        (0, M::Slappys) if view.selected < row => down,
        (0, M::Slappys) => a,
        (0, M::PartThumbnails) => {
            if !SHOP_THUMBS.swap(true, Ordering::Relaxed) {
                let rows: Vec<_> = view.rows.iter().map(|r| (r.title.as_str(), r.status.as_str(), r.cost, r.is_new)).collect();
                eprintln!("diagnostic: shop thumbnails (wallet {}) {rows:?}", view.wallet);
            }
            if spree_mode && !spree {
                SHOP_PHASE.store(1, Ordering::Relaxed);
                return b;
            }
            if !spree && let Some(selected) = view.rows.get(view.selected).filter(|r| r.status == "purchased") {
                eprintln!("diagnostic: shop added {} for {} (wallet {})", selected.title, selected.cost, view.wallet);
                SHOP_PHASE.store(1, Ordering::Relaxed);
                return b;
            }
            match view.rows.iter().position(|r| r.status == "none") {
                Some(target) if target / 3 > view.selected / 3 => down,
                Some(target) if target / 3 < view.selected / 3 => up,
                Some(target) if target % 3 > view.selected % 3 => right,
                Some(target) if target % 3 < view.selected % 3 => left,
                Some(_) => a,
                None => {
                    eprintln!("diagnostic: shop has nothing for sale here");
                    SHOP_PHASE.store(3, Ordering::Relaxed);
                    b
                }
            }
        }
        (0 | 1, _) => b,
        _ => start,
    }
}

fn demo_y_sample(held: bool) -> XboxPadSample {
    XboxPadSample {
        buttons: if held { 1 << 15 } else { 0 },
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    }
}

/// Exact raw A sample used by the original-data selected push corpus.
fn demo_push_sample() -> XboxPadSample {
    XboxPadSample {
        buttons: 1 | 0x1000,
        ..neutral_demo_sample()
    }
}

/// Inverts the source radial stick conversion for an authored pattern point.
/// This is diagnostic input synthesis, not gameplay physics or trick matching.
fn demo_right_point(point: [f32; 2]) -> XboxPadSample {
    let magnitude = point[1].mul_add(point[1], point[0] * point[0]).sqrt();
    let axis = |value: f32| -> i16 {
        if magnitude == 0.0 {
            return 0;
        }
        let raw_radius = 0.25 + magnitude / f32::from_bits(0x3fb6_db6e);
        ((value / magnitude * raw_radius * 32768.0).round()).clamp(-32768.0, 32767.0) as i16
    };
    XboxPadSample {
        right_x: axis(point[0]),
        right_y: axis(-point[1]),
        ..neutral_demo_sample()
    }
}

impl LiveSkatingClient {
    fn initialize(&mut self, view: &mut LiveSkatingView, renderer: &GpuRenderer) -> RenderResult<GpuScene> {
        renderer.set_display_settings(self.session.options_snapshot().values);
        let mut scene = view.initialize(renderer)?;
        if let Some(catalogue) = view.music_player_song_ids() {
            self.session.set_music_catalogue(catalogue)?;
        }
        if let Some(audio) = view.audio_interface() {
            self.session.set_audio_interface(audio);
        }
        if self.session.main_menu_focus()?.is_some() {
            // Controlled boot can return Pending over several redraws. Keep
            // the retained gameplay scene covered from the very first present;
            // frame() reveals only after update_main_menu replaces it with the
            // fully prepared ICE scene. Reuse the existing fullscreen cover.
            renderer.set_frontend_fade(&mut scene, 1.0)?;
        }
        Ok(scene)
    }

    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()> {
        if self.focused && self.session.extras_text_entry_active()
            && let WindowEvent::KeyboardInput { event, .. } = event
            && event.state == winit::event::ElementState::Pressed
        {
            use skate2_core::ui::extras::TextInput;
            use winit::keyboard::{Key, NamedKey};
            let input = match &event.logical_key {
                Key::Named(NamedKey::Enter) if !event.repeat => Some(TextInput::Accept),
                Key::Named(NamedKey::Escape) if !event.repeat => Some(TextInput::Cancel),
                Key::Named(NamedKey::Backspace) => Some(TextInput::Backspace),
                Key::Named(NamedKey::Delete) => Some(TextInput::Delete),
                Key::Named(NamedKey::ArrowLeft) => Some(TextInput::Left),
                Key::Named(NamedKey::ArrowRight) => Some(TextInput::Right),
                Key::Named(NamedKey::Home) => Some(TextInput::Home),
                Key::Named(NamedKey::End) => Some(TextInput::End),
                // winit names the space bar rather than reporting it as a character.
                Key::Named(NamedKey::Space) => Some(TextInput::Insert(" ".to_owned())),
                Key::Character(text) => Some(TextInput::Insert(text.to_string())),
                _ => None,
            };
            if let Some(input) = input { self.session.extras_text_input(input); }
        }
        // T opens the lobby's chat (PC port addition); the press opening it types nothing.
        if self.focused && !self.session.extras_text_entry_active()
            && let WindowEvent::KeyboardInput { event, .. } = event
            && event.state == winit::event::ElementState::Pressed && !event.repeat
            && event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyT)
        {
            self.session.online_open_chat();
        }
        if let WindowEvent::Focused(focused) = event {
            self.focused = *focused;
            if std::env::var_os("SKATE2_PROFILE_REDRAW").is_some() {
                eprintln!(
                    "diagnostic live focus: focused={focused} completed_frames={}",
                    self.completed_frames
                );
            }
        }
        Ok(())
    }

    fn completion_reached(&self) -> bool {
        if let Some(probe)=&self.replay_probe {return probe.done;}
        if matches!(self.controls_demo,Some(ControlsDemo::DisplaySettings)) {
            let end = if std::env::var_os("SKATE2_DIAGNOSTIC_DISPLAY_WALL").is_some() {830} else {1500};
            return self.frontend_demo_ticks >= end;
        }
        if matches!(self.controls_demo,Some(ControlsDemo::MainMenu)){
            let end=std::env::var("SKATE2_DIAGNOSTIC_FRONTEND_END").ok().and_then(|end|end.parse().ok()).unwrap_or(600);
            return self.frontend_demo_ticks>=end;
        }
        if matches!(self.controls_demo,Some(ControlsDemo::MainMenuRoundtrip)){return self.frontend_demo_ticks>=1500;}
        if matches!(self.controls_demo,Some(ControlsDemo::MainMenuGameplay)){return self.completed_frames>=900;}
        if matches!(self.controls_demo,Some(ControlsDemo::MainMenuFreeskate)){
            if let Ok(end) = std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY") {
                return self.frontend_demo_ticks >= end.parse().unwrap_or(1700);
            }
            let screen = std::env::var("SKATE2_DIAGNOSTIC_FREESKATE_SCREEN").ok().map(|tick| tick.parse().unwrap_or(1250));
            let end = if let Some(tick) = screen {tick}
                else if std::env::var_os("SKATE2_DIAGNOSTIC_FREESKATE_BACK").is_some() {1710}
                else if std::env::var_os("SKATE2_DIAGNOSTIC_FREESKATE_QUIT").is_some() {6500}
                else if let Ok(end) = std::env::var("SKATE2_DIAGNOSTIC_MUSIC_PLAYER_END") {end.parse().unwrap_or(3300)} else {2600};
            return self.frontend_demo_ticks>=end;
        }
        if matches!(self.controls_demo,Some(ControlsDemo::Career)){
            if let Ok(end)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_QUIT_END") {return self.frontend_demo_ticks>=end.parse().unwrap_or(6000);}
            if let Some(end)=std::env::var("SKATE2_DIAGNOSTIC_SUBTITLES_OPTION").ok().and_then(|tick|tick.parse::<u64>().ok()) {return self.frontend_demo_ticks>=end;}
            if std::env::var_os("SKATE2_DIAGNOSTIC_SUBTITLE_SHOT").is_some() {return subtitle_shot_done();}
            return self.completed_frames>=2400;
        }
        if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterPopup)) {
            return self.frontend_demo_ticks >= 840;
        }
        if matches!(self.controls_demo, Some(ControlsDemo::EditSkater)) {
            return self.frontend_demo_ticks >= 600;
        }
        if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterEyes | ControlsDemo::EditSkaterRoundtrip | ControlsDemo::EditSkaterCouch | ControlsDemo::EditSkaterStyles | ControlsDemo::EditSkaterGender | ControlsDemo::EditSkaterRemoveShirt | ControlsDemo::MapTeleport)) {
            return self.frontend_demo_ticks >= 1500;
        }
        self.completion_limit
            .is_some_and(|limit| self.completed_frames >= limit)
    }

    fn completion_bound_active(&self) -> bool {
        matches!(self.controls_demo,Some(ControlsDemo::MainMenu|ControlsDemo::MainMenuRoundtrip|ControlsDemo::MainMenuGameplay|ControlsDemo::MainMenuFreeskate|ControlsDemo::DisplaySettings|ControlsDemo::Career)) || self.completion_limit.is_some()
            || matches!(self.controls_demo, Some(ControlsDemo::EditSkater | ControlsDemo::EditSkaterEyes | ControlsDemo::EditSkaterRoundtrip | ControlsDemo::EditSkaterCouch | ControlsDemo::EditSkaterStyles | ControlsDemo::EditSkaterGender | ControlsDemo::EditSkaterPopup | ControlsDemo::EditSkaterRemoveShirt | ControlsDemo::MapTeleport))
    }

    fn record_presented_completion(&mut self) {
        self.presented_completion_counter
            .store(self.completed_frames, Ordering::Relaxed);
    }

    fn frame_presented(&mut self, alpha: f32) {
        self.session.acknowledge_frontend_frame_presented(alpha);
        if let Some(probe)=self.replay_probe.as_mut() {
            let pause=self.session.pause_menu_snapshot();
            probe.presented(self.session.replay_editor_snapshot().is_some_and(|s|s.ready),
                !self.session.replay_editor_open() && pause.visible && !pause.screen_hidden,alpha);
        }
        if std::env::var_os("SKATE2_TRACE_FRONTEND_FADE").is_some() && alpha > 0. {
            eprintln!("frontend fade presented: tick={} alpha={} editor={} paused={}",
                self.frontend_demo_ticks, alpha,
                self.session.edit_skater_view().is_ok_and(|v|v.is_some()),
                self.session.pause_menu_snapshot().visible);
        }
    }
    /// The serial host frame: every due tick, then this redraw's frontend work.
    fn frame(
        &mut self,
        view: &mut LiveSkatingView,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
    ) -> RenderResult<()> {
        // A capture may need another redraw after a surface timeout. Retain
        // the completed visual objects and never advance to the next tick.
        if self.completion_reached() {
            self.previous = now;
            return Ok(());
        }
        if !self.ready {
            let progress = self.session.try_poll_controlled_boot(
                &mut |_, _, _| Ok::<_, Infallible>(SelectedGestureAllocation::Host),
                &mut || panic!("source spawn orthogonality assertion"),
                &mut || panic!("hat-enable host answer is not reconstructed"),
            )?;
            if self.last_progress != Some(progress) {
                eprintln!("controlled first-test boot={progress:?}");
                self.last_progress = Some(progress);
            }
            if progress != SelectedBootProgress::ControlledFrameReady {
                self.previous = now;
                return Ok(());
            }
            self.ready = true;
            if self.ollie_demo {
                let points = self.session.controlled_right_pattern_points(&self.ollie_pattern);
                self.ollie_points =
                    Some(points.try_into().map_err(
                        |_| "selected original right-stick pattern must contain two points",
                    )?);
                eprintln!(
                    "live trick demo: replaying original {} right-stick pattern", self.ollie_pattern
                );
            } else if self.y_demo {
                eprintln!(
                    "live Y demo: pressing raw FaceY at selected frame index 32, then releasing"
                );
            } else if self.push_demo {
                eprintln!(
                    "live push demo: replaying the original-data raw A sample at corpus steps 32..43 after one neutral boot frame"
                );
            }
            // Boot work precedes simulation. Start the clock and its diagnostic
            // at the same instant instead of charging boot time to catch-up.
            let ready_now = Instant::now();
            self.previous = ready_now;
            self.controlled_run_start = Some(ready_now);
            self.accumulator = FIXED_STEP;
            eprintln!(
                "controlled frame ready; fixed host simulation step 1/60, source-selected original-data input/Actor/physics/camera/palette; camera view and map shading remain diagnostic"
            );
        } else {
            self.advance_clock(now);
        }
        // Bound event-loop work, but retain every unconsumed wall-clock slot.
        // Each try_tick below polls through the same native queued input path;
        // it does not reconstruct input history for elapsed wall-clock slots.
        let mut ticks_this_redraw = 0;
        while tick_due(self.accumulator, self.step, ticks_this_redraw) {
            ticks_this_redraw += 1;
            self.accumulator -= self.step;
            let was_paused = self.stage_tick(&view.navigation())?;
            let record = self.run_tick(was_paused);
            if self.complete_tick(view, renderer, scene, now, record, None)? {
                break;
            }
        }
        self.finish_redraw(view, renderer, scene, now, 0)?;
        Ok(())
    }

    fn advance_clock(&mut self, now: Instant) {
        if now > self.previous {
            self.accumulator = self.accumulator.saturating_add(now - self.previous);
            self.previous = now;
        }
    }

    /// Gameplay ticks may run on the SIM thread. Boot, frontend modes and the
    /// completion bound keep the serial per-redraw interleaving.
    fn threaded_tick_allowed(&self) -> bool {
        // Net::ResumeGame: an online game keeps simulating under the pause menu and Options, so
        // those frames take the gameplay path and its restored frames between ticks.
        let simulating = self.session.online_simulates_under_frontend();
        self.ready
            && self.gameplay
            && !self.completion_reached()
            && (simulating || !self.session.pause_menu_snapshot().visible)
            && !self.session.replay_editor_open()
            && (simulating
                || self.session.options_snapshot().page
                    == skate2_core::ui::options::Page::Hidden)
            && (simulating || !self.session.music_player_snapshot().visible)
            && self.session.edit_skater_view().is_ok_and(|view| view.is_none())
            && self.session.main_menu_frame().is_ok_and(|frame| frame.is_none())
    }

    fn audio_inputs(&self)->(skate2_audio::system::GameAudioState,Option<crate::game_audio::AudioWorldInput>) {
        // The front-end state UpdateFEAudioState last reported: the pause and career screens on top,
        // free skate's HUD state otherwise.
        let frontend = self.session.frontend_audio_state().unwrap_or(skate2_core::ui::FrontendAudioState::FREE_SKATE);
        let exporting=self.session.replay_export_active();
        let audio_frame = if self.session.audio_is_fe()&&!exporting { None } else { self.session.audio_frame() };
        let (challenge_type, challenge_host, challenge_large_arena, game_mode) =
            audio_frame.as_ref().map_or_else(|| self.session.audio_challenge(), |frame| frame.challenge);
        let (gvr, danny_way_jam, mega_challenge) =
            audio_frame.as_ref().map_or_else(|| self.session.audio_challenge_names(), |frame| frame.challenge_names);
        let (player_providing_input, player_is_male) =
            audio_frame.as_ref().map_or_else(|| self.session.audio_player_flags(), |frame| frame.player_flags);
        let (in_replay, playback_speed) = self.session.audio_playback();
        let audio_state = skate2_audio::system::GameAudioState {
            presentation: Some((in_replay, playback_speed.abs())),
            hud_state: frontend.state,
            fe_music_state: frontend.music_state,
            paused: self.session.audio_paused(),
            exiting_to_main_menu: self.session.exiting_to_main_menu(),
            is_fe: i32::from(self.session.audio_is_fe()&&!exporting),
            nis_fade_amount: self.session.audio_nis_fade_amount(),
            sfx_volume: self.session.profile_settings().sfx_volume,
            speech_volume: self.session.profile_settings().dialogue_volume,
            music_volume: self.session.profile_settings().music_volume,
            music_mode: self.session.music_mode(),
            new_profile: self.session.new_profile(),
            challenge_type,
            challenge_host,
            challenge_large_arena,
            game_mode,
            mega_challenge,
            gvr,
            danny_way_jam,
            player_providing_input,
            player_is_male,
            online_game: self.session.audio_online_game(),
            ..Default::default()
        };
        // The front end's simulation keeps running; DoSimSubmit submits its AI skaters.
        let audio_world = if let Some(front_end) = if exporting {None} else {self.session.audio_front_end()} {
            Some(crate::game_audio::AudioWorldInput {
                security: Default::default(),
                game_speed: 1.0,
                is_rewinding: false,
                frame: front_end.frame,
                camera_at: front_end.camera_at,
                camera_position: front_end.camera_position,
                skaters: Some(self.session.audio_front_end_skaters()),
                front_end: true,
                reverb_key: front_end.reverb_key,
                ambience_key: front_end.ambience_key,
                emitter_key: front_end.emitter_key,
                out_of_boundary_amount: None,
                fade_amount: None,
                pedestrians: Vec::new(),
                vehicles: Vec::new(),
                dynamic_objects: Vec::new(),
                local_race_ranking: -1,
            })
        } else { audio_frame.map(|frame| crate::game_audio::AudioWorldInput {
            security: frame.security,
            game_speed: playback_speed.abs(),
            is_rewinding: self.session.audio_rewinding(),
            frame: frame.frame,
            camera_at: frame.camera_at,
            camera_position: frame.camera_position,
            skaters: Some(frame.skaters),
            front_end: false,
            reverb_key: frame.reverb_key,
            ambience_key: frame.ambience_key,
            emitter_key: frame.emitter_key,
            out_of_boundary_amount: frame.out_of_boundary_amount,
            fade_amount: frame.fade_amount,
            pedestrians: frame.pedestrians,
            vehicles: frame.vehicles,
            dynamic_objects: frame.dynamic_objects,
            local_race_ranking: frame.local_race_ranking,
        }) };
        (audio_state,audio_world)
    }

    /// Timer re-arm, scripted pad samples and the presentation navigation of
    /// the previous completed view, staged before one tick.
    fn stage_tick(
        &mut self,
        navigation: &[skate2_core::presentation::PresentationNavigationArena],
    ) -> RenderResult<bool> {
        // timerThread8298a2c4..a4f0 re-arms from the period current at this
        // fire, before the woken tick runs; a slow-motion rate that tick
        // sets paces the fire after next.
        if self.session.pause_menu_snapshot().visible {
            self.step = FIXED_STEP;
        } else if let Some(hz) = self.session.sim_timer_hz() {
            self.step = timer_period(hz);
        }
        if let Some([antic, flick]) = self.ollie_points {
            // Opt-in scoring diagnostic repeats ONLY original raw stick
            // samples; physics, trick recognition and scoring stay live.
            let demo_tick = self.ollie_period.map_or(self.completed_frames,
                |period| self.completed_frames % period);
            let flick_start = 7 + self.ollie_hold;
            let mut sample = if (6..6 + self.ollie_hold).contains(&demo_tick) {
                demo_right_point(antic)
            } else if (flick_start..flick_start + 2).contains(&demo_tick) {
                demo_right_point(flick)
            } else {
                neutral_demo_sample()
            };
            // Diagnostic raw input only; held left stick enters the normal
            // input/Action/physics path, without forcing angular velocity.
            if self.ollie_spin
                && self
                    .session
                    .diagnostic_state_ids()
                    .is_some_and(|(_, _, p)| p == 200 || p == 201)
            {
                sample.left_x = if self.ollie_spin_left {
                    i16::MIN
                } else {
                    i16::MAX
                };
            }
            // The follow-up left-stick scenario holds through landing too.
            if self.ollie_spin_left && (flick_start..180).contains(&demo_tick) {
                sample.left_x = i16::MIN;
            }
            if self.ollie_tweak_left
                && self
                    .session
                    .diagnostic_state_ids()
                    .is_some_and(|(_, _, p)| p == 200 || p == 201)
            {
                sample.right_x = i16::MIN;
                sample.right_y = 0;
            }
            self.pad.demo_sample = Some(sample);
        } else if let Some(demo) = self.controls_demo {
            let mut sample = demo.sample(self.completed_frames);
            if matches!(demo,ControlsDemo::MainMenu|ControlsDemo::MainMenuRoundtrip|ControlsDemo::MainMenuGameplay|ControlsDemo::MainMenuFreeskate|ControlsDemo::DisplaySettings|ControlsDemo::Career){
                let tick=self.frontend_demo_ticks;
                // SKATE2_DIAGNOSTIC_TRAVEL=x,y,z,yaw[;x,y,z,yaw...] at SKATE2_DIAGNOSTIC_TRAVEL_TICK=
                // tick[;tick...]: the map's travel to each place at its tick, facing yaw radians from +z.
                if let (Ok(ticks), Ok(travels))=(std::env::var("SKATE2_DIAGNOSTIC_TRAVEL_TICK"), std::env::var("SKATE2_DIAGNOSTIC_TRAVEL"))
                    && let Some(travel)=ticks.split(';').zip(travels.split(';')).find(|(at, _)| at.trim().parse::<u64>().ok()==Some(tick)).map(|(_, travel)| travel)
                {
                    let values: Vec<f32> = travel.split(',').filter_map(|value| value.trim().parse().ok()).collect();
                    if let [x, y, z, yaw] = values[..] {
                        eprintln!("diagnostic: travel to {x},{y},{z} yaw {yaw} at {tick}");
                        self.session.diagnostic_travel([x, y, z], yaw);
                    }
                }
                // SKATE2_DIAGNOSTIC_CASH=<amount> at SKATE2_DIAGNOSTIC_CASH_TICK: the career's AddCash.
                if std::env::var("SKATE2_DIAGNOSTIC_CASH_TICK").ok().and_then(|tick|tick.parse::<u64>().ok())==Some(tick)
                    && let Some(cash)=std::env::var("SKATE2_DIAGNOSTIC_CASH").ok().and_then(|cash|cash.parse::<i32>().ok())
                {
                    eprintln!("diagnostic: add cash {cash} at {tick}: {}", self.session.diagnostic_add_cash(cash));
                }
                // Original-data UI inspection: open a HoM screen using the recorded bail.
                if let Ok(entries)=std::env::var("SKATE2_DIAGNOSTIC_HOM_SCREEN") {
                    for entry in entries.split(';') {
                        if let Some((at,screen))=entry.split_once(':')
                            && at.parse::<u64>().ok()==Some(tick) {
                            self.session.diagnostic_hom_screen(screen=="annals");
                        }
                    }
                }
                // SKATE2_DIAGNOSTIC_ACTIVATE_CHALLENGE=<title> at SKATE2_DIAGNOSTIC_ACTIVATE_TICK: the
                // progression's ActivateChallengeByTitle.
                if std::env::var("SKATE2_DIAGNOSTIC_ACTIVATE_TICK").ok().and_then(|tick|tick.parse::<u64>().ok())==Some(tick)
                    && let Ok(title)=std::env::var("SKATE2_DIAGNOSTIC_ACTIVATE_CHALLENGE")
                {
                    eprintln!("diagnostic: activate {title} at {tick}: {}", self.session.diagnostic_activate_challenge(&title));
                }
                // SKATE2_DIAGNOSTIC_PROFILE_NAME=<name> (and _ONLINE_NICKNAME): typed into the
                // first-use name prompt (the Online nickname prompt), then Enter.
                if tick % 30 == 0 && let Some(frame) = self.session.main_menu_frame()? {
                    let name = if frame.name_prompt.is_some() { std::env::var("SKATE2_DIAGNOSTIC_PROFILE_NAME").ok() }
                        else if frame.nickname_prompt.is_some() { std::env::var("SKATE2_DIAGNOSTIC_ONLINE_NICKNAME").ok() }
                        else { None };
                    if let Some(name) = name {
                        use skate2_core::ui::extras::TextInput;
                        self.session.extras_text_input(TextInput::Insert(name));
                        self.session.extras_text_input(TextInput::Accept);
                    }
                }
                if matches!(demo,ControlsDemo::Career) {
                    if tick==600 { sample.buttons=0x1000; }
                    if tick==0 && std::env::var_os("SKATE2_TRACE_SUBTITLES").is_some() {
                        eprintln!("subtitles option: profile loaded with {}", self.session.profile_settings().subtitles);
                    }
                    // SKATE2_DIAGNOSTIC_SUBTITLES_OPTION[=<tick>]: the main menu's Options (row 5), Video
                    // Settings (A at 600), its fifth row (Subtitles), right, Back twice (the profile
                    // save), then New Career at 900; a tick ends the demo there.
                    if std::env::var_os("SKATE2_DIAGNOSTIC_SUBTITLES_OPTION").is_some() {
                        match tick {
                            360|390|420|450|480|570|630|660|690|720 => sample.buttons=0x0002,
                            540|900 => sample.buttons=0x1000,
                            750 => sample.buttons=0x0008,
                            780|810 => sample.buttons=0x2000,
                            840 => {
                                let store=skate2_core::edit_skater_profile::EditSkaterProfileStore::open(
                                    skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?)?;
                                eprintln!("subtitles option: SKATER.P holds {}, the profile {}",
                                    store.load_settings()?.subtitles, self.session.profile_settings().subtitles);
                            }
                            _ => {}
                        }
                    }
                    // SKATE2_DIAGNOSTIC_SHOP=<Slappy's row>: the editor shops instead of leaving
                    // at once; SKATE2_DIAGNOSTIC_SHOP_REOPEN=<tick> opens it again from the hub.
                    let shop_row = std::env::var("SKATE2_DIAGNOSTIC_SHOP").ok().and_then(|row| row.parse::<usize>().ok());
                    let view = self.session.edit_skater_view()?;
                    if let Some(row) = shop_row {
                        if tick>600 && tick%30==0 && let Some(view) = &view { sample.buttons=shop_step(view, row); }
                        shop_session(view.is_some());
                        let reopen = std::env::var("SKATE2_DIAGNOSTIC_SHOP_REOPEN").ok().and_then(|tick| tick.parse::<u64>().ok());
                        if reopen==Some(tick) { sample.buttons=0x0010; }
                        let pause = self.session.pause_menu_snapshot();
                        if reopen.is_some_and(|at| tick>at) && tick%30==0 && pause.visible && view.is_none() {
                            sample.buttons = if pause.options()[pause.selected] == skate2_core::ui::pause_menu::PauseMenuOption::EditSkater { 0x1000 } else { 0x0002 };
                        }
                    } else if tick>600 && tick%60==0 && let Some(view) = &view {
                        // Start finishes; A answers the exit confirmation's default Yes.
                        sample.buttons = if view.exit_confirmation.is_some() { 0x1000 } else { 0x0010 };
                    }
                    // A at the given tick: the "New to skate?" popup's default No.
                    if std::env::var("SKATE2_DIAGNOSTIC_CAREER_ANSWER").ok().and_then(|tick|tick.parse::<u64>().ok())==Some(tick) {
                        sample.buttons=0x1000;
                    }
                    // SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT=<challenge>,<challenge>[,...] at
                    // SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT_TICK: the sign-up prompt for those challenges.
                    if std::env::var("SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT_TICK").ok().and_then(|tick|tick.parse::<u64>().ok())==Some(tick)
                        && let Ok(names)=std::env::var("SKATE2_DIAGNOSTIC_SIGN_UP_PROMPT")
                    {
                        let names: Vec<&str> = names.split(',').map(str::trim).collect();
                        eprintln!("diagnostic: sign-up prompt {names:?} at {tick}: {}", self.session.diagnostic_sign_up_prompt(&names));
                    }
                    // The diagnostics up to SKATE2_DIAGNOSTIC_CAREER_PAD_AFTER count their ticks from the
                    // first tick with a career.
                    static CAREER_START: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
                    if CAREER_START.load(Ordering::Relaxed) == u64::MAX && self.session.career_local_player().is_some() {
                        CAREER_START.store(tick, Ordering::Relaxed);
                        eprintln!("diagnostic: career from {tick}");
                    }
                    let career_tick = tick.checked_sub(CAREER_START.load(Ordering::Relaxed));
                    let in_range = |range: &str| range.split_once('-').is_some_and(|(from, to)| {
                        career_tick.is_some_and(|at| from.trim().parse::<u64>().is_ok_and(|from| at >= from)
                            && to.trim().parse::<u64>().is_ok_and(|to| at < to))
                    });
                    // SKATE2_DIAGNOSTIC_COMPLETE_CHALLENGES=<tick>:<name>[,<name>...][;...]: the signpost's
                    // CompleteChallenge of each named progression challenge at the tick.
                    if let Ok(list)=std::env::var("SKATE2_DIAGNOSTIC_COMPLETE_CHALLENGES") {
                        for entry in list.split(';') {
                            if let Some((at, names))=entry.split_once(':')
                                && at.trim().parse::<u64>().is_ok_and(|at| Some(at)==career_tick)
                            {
                                for name in names.split(',').map(str::trim) {
                                    eprintln!("diagnostic: complete {name} at {tick}: {}", self.session.diagnostic_complete_challenge(name));
                                }
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_SIGNPOST_MESSAGE=<tick>:<message>[;...]: the progression message sent
                    // at its tick, after the text setting SKATE2_DIAGNOSTIC_SIGNPOST_SETTING=<setting>=<text>.
                    if let Ok(list)=std::env::var("SKATE2_DIAGNOSTIC_SIGNPOST_MESSAGE") {
                        for entry in list.split(';') {
                            if let Some((at, message))=entry.split_once(':')
                                && at.trim().parse::<u64>().is_ok_and(|at| Some(at)==career_tick)
                            {
                                let setting=std::env::var("SKATE2_DIAGNOSTIC_SIGNPOST_SETTING").ok();
                                let setting=setting.as_deref().and_then(|setting| setting.split_once('='));
                                eprintln!("diagnostic: signpost message {message} at {tick}: {}",
                                    self.session.diagnostic_send_signpost_message(setting, message.trim()));
                            }
                        }
                    }
                    // Raw stick input for repeatable bail/prop interaction inspection;
                    // ranges use the same career clock as PAD_AFTER and TRIGGERS.
                    // SKATE2_DIAGNOSTIC_CAREER_STICKS=from-to:lx,ly,rx,ry[;...].
                    if let Ok(entries)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_STICKS") {
                        for entry in entries.split(';') {
                            if let Some((range,axes))=entry.split_once(':') && in_range(range) {
                                let values:Vec<i16>=axes.split(',').filter_map(|v|v.trim().parse().ok()).collect();
                                if let [lx,ly,rx,ry]=values[..] {
                                    sample.left_x=lx;sample.left_y=ly;sample.right_x=rx;sample.right_y=ry;
                                }
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CAREER_TRIGGERS=<from>-<to>:<l|r>[;...]: the left or right trigger
                    // pulled over those ticks.
                    if let Ok(entries)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_TRIGGERS") {
                        for entry in entries.split(';') {
                            if let Some((range, side)) = entry.split_once(':')
                                && in_range(range)
                            {
                                if side.trim() == "l" { sample.left_trigger=255; } else { sample.right_trigger=255; }
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CAREER_PAD_AFTER=<from>-<to>:<hex buttons>[;...]: the buttons held
                    // over those ticks.
                    if let Ok(entries)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_PAD_AFTER") {
                        for entry in entries.split(';') {
                            if let Some((range, buttons)) = entry.split_once(':')
                                && in_range(range)
                                && let Ok(buttons) = u16::from_str_radix(buttons.trim(), 16)
                            {
                                sample.buttons=buttons;
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CAREER_PAD=<from>-<to>[:<hex buttons>][;...]: the buttons (A, a push,
                    // by default) held over those ticks.
                    if let Ok(entries)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_PAD") {
                        for entry in entries.split(';') {
                            let (range, buttons) = entry.split_once(':').unwrap_or((entry, "1000"));
                            if let Some((from, to)) = range.split_once('-')
                                && from.trim().parse::<u64>().is_ok_and(|from| tick>=from)
                                && to.trim().parse::<u64>().is_ok_and(|to| tick<to)
                                && let Ok(buttons) = u16::from_str_radix(buttons.trim(), 16)
                            {
                                sample.buttons=buttons;
                            }
                        }
                    }
                    // Bounded diagnostic route through authored global-activity boxes. Each
                    // tick:x,y,z uses the same actor teleport as the existing race diagnostic;
                    // normal layer/trigger/activity updates perform the subsequent transitions.
                    if let Ok(route) = std::env::var("SKATE2_DIAGNOSTIC_CAREER_TELEPORT") {
                        for entry in route.split(';') {
                            if let Some((at, position)) = entry.split_once(':')
                                && at.trim().parse::<u64>().is_ok_and(|at| at == tick)
                            {
                                if let Some(name) = position.strip_prefix('@') {
                                    eprintln!("diagnostic: route location {name} at {tick}: {}",
                                        self.session.diagnostic_teleport_named_location(name));
                                    continue;
                                }
                                let values: Vec<f32> = position.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                                if let [x, y, z] = values[..] {
                                    eprintln!("diagnostic: route teleport {values:?} at {tick}: {}",
                                        self.session.diagnostic_teleport_local_player([x, y, z]));
                                }
                            }
                        }
                    }
                    // Scoped menu inspection through ordinary pad input: a comma-separated path
                    // of category/HAL strings, stopping on the info panel for capture/Teleport.
                    // A '*' part selects the current row where nothing else matches;
                    // SKATE2_DIAGNOSTIC_MAP_TELEPORT presses Teleport on the info panel.
                    if let Ok(path)=std::env::var("SKATE2_DIAGNOSTIC_MAP_PATH") {
                        let from=std::env::var("SKATE2_DIAGNOSTIC_MAP_TICK").ok().and_then(|n|n.parse::<u64>().ok()).unwrap_or(1200);
                        static DONE: AtomicBool=AtomicBool::new(false);
                        if tick>=from && tick%60==0 && !DONE.load(Ordering::Relaxed){
                            let map=self.session.map_menu_snapshot();
                            let pause=self.session.pause_menu_snapshot();
                            if map.visible {
                                if let Some(view)=map.career.as_deref(){
                                    if let Some(info)=&view.info {
                                        eprintln!("diagnostic: map info at {tick}: {info:?}");
                                        DONE.store(true,Ordering::Relaxed);
                                        if std::env::var_os("SKATE2_DIAGNOSTIC_MAP_TELEPORT").is_some() {
                                            sample.buttons=0x1000;
                                        }
                                    }else{
                                        use skate2_core::challenge::stuff_menu::MenuText;
                                        let target=view.rows.iter().position(|row|{
                                            let text=match row {MenuText::Copied(s)|MenuText::Formatted(s)=>s.as_str(),MenuText::Category{hal,..}=>*hal,MenuText::Empty=>""};
                                            path.split(',').any(|part|part==text)
                                        });
                                        eprintln!("diagnostic: map at {tick} title={:?} rows={:?} selected={} target={target:?}",view.title,view.rows,view.selected);
                                        if let Some(target)=target {
                                            sample.buttons=match view.selected.cmp(&target){std::cmp::Ordering::Less=>0x0002,std::cmp::Ordering::Greater=>0x0001,std::cmp::Ordering::Equal=>0x1000};
                                        } else if path.split(',').any(|part|part=="*") {
                                            sample.buttons=0x1000;
                                        }
                                    }
                                }
                            }else if pause.visible {
                                sample.buttons=if pause.options()[pause.selected]==skate2_core::ui::pause_menu::PauseMenuOption::Map{0x1000}else{0x0002};
                            }else{sample.buttons=0x0010;}
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CHALLENGE at SKATE2_DIAGNOSTIC_CHALLENGE_TICK: sign the
                    // named challenge up, then A on its poster.
                    if let Ok(name)=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE") {
                        let at=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE_TICK").ok().and_then(|tick|tick.parse::<u64>().ok());
                        if at==Some(tick) {
                            eprintln!("diagnostic: sign up {name}: {}", self.session.diagnostic_sign_up_challenge(&name));
                        }
                        if at.is_some_and(|at|tick>at) && tick%30==0 && self.session.career_poster_ready()
                            && std::env::var_os("SKATE2_DIAGNOSTIC_POSTER_MANUAL").is_none() {
                            sample.buttons=0x1000;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_MISSION_MUSIC=<tick>:<trigger>:<challenge>[;...]: the named
                    // challenge's ChallengeMusic.PlayMusic(trigger) at the tick.
                    if let Ok(list)=std::env::var("SKATE2_DIAGNOSTIC_MISSION_MUSIC") {
                        for entry in list.split(';') {
                            let mut parts=entry.splitn(3, ':');
                            if let (Some(at), Some(trigger), Some(name))=(parts.next(), parts.next(), parts.next())
                                && at.trim().parse::<u64>().is_ok_and(|at|at==tick)
                                && let Ok(trigger)=trigger.trim().parse::<i32>()
                            {
                                eprintln!("diagnostic: mission music {trigger} for {name} at {tick}: {}",
                                    self.session.diagnostic_trigger_mission_music(trigger, name.trim()));
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CAREER_YES: Yes on the popup, 90 ticks after it opens.
                    if std::env::var_os("SKATE2_DIAGNOSTIC_CAREER_YES").is_some() && self.session.career_dialog_open() {
                        static OPENED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                        let opened = match OPENED.load(Ordering::Relaxed) { 0 => { OPENED.store(tick, Ordering::Relaxed); tick } opened => opened };
                        if tick == opened {
                            eprintln!("diagnostic: popup open at {tick}, skater (x, z) {:?}", self.session.map_player_pose().map(|(position, _)| position));
                        }
                        if tick == opened + 90 { sample.buttons = 0x0001; }
                        if tick == opened + 150 { sample.buttons = 0x1000; }
                    }
                    // SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES: from the given tick, every 240 ticks the main
                    // challenge's active objectives are forced complete; A halfway between for popups.
                    let force_end=std::env::var("SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES_END").ok().and_then(|tick|tick.parse::<u64>().ok()).unwrap_or(u64::MAX);
                    if let Some(from)=std::env::var("SKATE2_DIAGNOSTIC_FORCE_OBJECTIVES").ok().and_then(|tick|tick.parse::<u64>().ok())
                        && tick>=from && tick<force_end {
                        if (tick-from)%240==0 {
                            let forced=self.session.diagnostic_force_objectives();
                            if !forced.is_empty() { eprintln!("diagnostic: forced {forced:?} at {tick}"); }
                        }
                        if (tick-from)%240==120 { sample.buttons=0x1000; }
                    }
                    // SKATE2_DIAGNOSTIC_CHALLENGE2 at SKATE2_DIAGNOSTIC_CHALLENGE2_TICK: a second challenge
                    // signed up the same way, then A on its poster.
                    if let Ok(name)=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE2") {
                        let at=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE2_TICK").ok().and_then(|tick|tick.parse::<u64>().ok());
                        if at==Some(tick) {
                            eprintln!("diagnostic: sign up {name}: {}", self.session.diagnostic_sign_up_challenge(&name));
                        }
                        if at.is_some_and(|at|tick>at) && tick%30==0 && self.session.career_poster_ready()
                            && std::env::var_os("SKATE2_DIAGNOSTIC_POSTER_MANUAL").is_none() {
                            sample.buttons=0x1000;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP=a,b,...: from SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_TICK
                    // (default 3000), one challenge per SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_PERIOD ticks
                    // (default 1200): the running main challenge is abandoned, the next is signed up 60
                    // ticks later and its poster, popups and end options take A; 30 ticks before the
                    // next step the main challenge and its state are logged. `+name` runs a global
                    // activity instead, `!name` sends a running teleporter its sign-up.
                    if let Ok(names)=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP") {
                        let names: Vec<&str> = names.split(',').map(str::trim).filter(|name| !name.is_empty()).collect();
                        let from=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_TICK").ok().and_then(|tick|tick.parse::<u64>().ok()).unwrap_or(3000);
                        let period=std::env::var("SKATE2_DIAGNOSTIC_CHALLENGE_SWEEP_PERIOD").ok().and_then(|tick|tick.parse::<u64>().ok()).unwrap_or(1200).max(240);
                        if let Some(elapsed)=tick.checked_sub(from) {
                            let (step, phase) = (usize::try_from(elapsed / period).unwrap_or(usize::MAX), elapsed % period);
                            if phase==0 && step>0 && step<=names.len() {
                                eprintln!("diagnostic: sweep abandon at {tick}: {}", self.session.diagnostic_abandon_challenge());
                            }
                            static SIGNED_STEP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                            if phase>=60 && phase<period-30 && phase%30==0
                                && SIGNED_STEP.load(Ordering::Relaxed) != (step as u64 + 1)
                                && let Some(name)=names.get(step) {
                                let started=if let Some(global)=name.strip_prefix('+') {
                                    self.session.diagnostic_run_global_activity(global)
                                } else if let Some(teleporter)=name.strip_prefix('!') {
                                    self.session.diagnostic_teleporter_sign_up(teleporter)
                                } else {
                                    self.session.diagnostic_sign_up_challenge(name)
                                };
                                if started { SIGNED_STEP.store(step as u64 + 1, Ordering::Relaxed); }
                                eprintln!("diagnostic: sweep {step} sign up {name} at {tick}: {started}");
                            }
                            if phase==period-30 && let Some(name)=names.get(step) {
                                eprintln!("diagnostic: sweep {step} {name} main {:?} at {tick}", self.session.career_main_challenge());
                            }
                            if phase>60 && step<names.len() && tick%30==0 && self.session.career_poster_ready() {
                                sample.buttons=if self.session.career_poster_affordable() { 0x1000 } else { 0x2000 };
                            }
                            if phase>60 && tick%90==15 && (self.session.career_end_options_ready() || self.session.career_dialog_open() || self.session.career_overlay_open()) {
                                sample.buttons=0x1000;
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_PHOTO_SEQUENCE: from the given tick, the local player's physics
                    // messages of m_1a_05's rail photo (as the headless test sends them): the sequence
                    // begins, the spot volume is entered, a 3 m grind of the rail, the sequence completes.
                    if let Some(at)=std::env::var("SKATE2_DIAGNOSTIC_PHOTO_SEQUENCE").ok().and_then(|tick|tick.parse::<u64>().ok()) {
                        use skate2_core::challenge::{Event, system::{Field, Message}};
                        let local = |name: &str, mut fields: Vec<(&'static str, Field)>| {
                            fields.insert(0, ("PlayerID", Field::Number(0.0)));
                            Event::Challenge(Message::new(name, fields))
                        };
                        let rail = Field::Key(450_138_407_667_463);
                        // SKATE2_DIAGNOSTIC_PHOTO_SPOT=x,y,z: the player on that spot a second
                        // before, so the photo camera has the skater in view.
                        if tick + 60 == at
                            && let Ok(spot) = std::env::var("SKATE2_DIAGNOSTIC_PHOTO_SPOT")
                        {
                            let values: Vec<f32> = spot.split(',').filter_map(|value| value.trim().parse().ok()).collect();
                            if let [x, y, z] = values[..] {
                                eprintln!("diagnostic: photo spot teleport {}", self.session.diagnostic_teleport_local_player([x, y, z]));
                            }
                        }
                        // SKATE2_DIAGNOSTIC_PHOTO_PUSH: from the teleport until the sequence
                        // completes, A held 12 ticks of every 30, so the replay has the skater
                        // rolling.
                        if std::env::var_os("SKATE2_DIAGNOSTIC_PHOTO_PUSH").is_some()
                            && tick + 60 > at
                            && tick < at + 92
                            && (tick + 60 - at) % 30 < 12
                        {
                            sample.buttons = 0x1000;
                        }
                        let event = match tick.checked_sub(at) {
                            Some(0) => Some(local("cMsgSequenceBegin", Vec::new())),
                            Some(1) => Some(Event::TriggerCollision { enter: true, volume: 1_414_882_551_629_574, player: 0 }),
                            Some(90) => Some(local("cMsgGrindCompleted", vec![
                                ("Wipedout", Field::Number(0.0)),
                                ("UsedSessionMarker", Field::Bool(false)),
                                ("GrindLength", Field::Number(3.0)),
                                ("PathedSplineID", rail.clone()),
                                ("LocalSplineID", rail),
                            ])),
                            Some(91) => Some(local("cMsgSequenceComplete", vec![
                                ("SequenceScore", Field::Number(600.0)),
                                ("StartVisualGameTick", Field::Number(0.0)),
                                ("StartTick", Field::Number(0.0)),
                                ("TotalTrickCount", Field::Number(1.0)),
                                ("StoredTrickCount", Field::Number(1.0)),
                                ("Wipedout", Field::Bool(false)),
                                ("UsedSessionMarker", Field::Bool(false)),
                                ("StartSimGameTick", Field::Number(0.0)),
                            ])),
                            _ => None,
                        };
                        if let Some(event) = event {
                            eprintln!("diagnostic: photo sequence {:?} at {tick}", tick - at);
                            self.session.diagnostic_post_challenge_event(event);
                        }
                    }
                    // SKATE2_DIAGNOSTIC_PHOTO_PICK: while a photo outro's photo selection is up, the right
                    // trigger 60 ticks after it opens, then Y (the pick) 60 ticks later.
                    if std::env::var_os("SKATE2_DIAGNOSTIC_PHOTO_PICK").is_some() {
                        static OPENED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
                        if self.session.career_photo_select_open() {
                            let opened = match OPENED.load(Ordering::Relaxed) { u64::MAX => { OPENED.store(tick, Ordering::Relaxed); tick } opened => opened };
                            if tick == opened + 60 { sample.right_trigger = u8::MAX; eprintln!("diagnostic: photo select next"); }
                            if tick == opened + 120 { sample.buttons = 0x8000; eprintln!("diagnostic: photo pick"); }
                        } else {
                            OPENED.store(u64::MAX, Ordering::Relaxed);
                        }
                        // Then Continue on the end-of-challenge options once their screen is ready,
                        // and A on the unlock overlay that follows.
                        if tick % 90 == 0 && self.session.career_end_options_ready() {
                            eprintln!("diagnostic: end options A at {tick}");
                            sample.buttons = 0x1000;
                        }
                        if tick % 90 == 45 && self.session.career_overlay_open() {
                            eprintln!("diagnostic: overlay A at {tick}");
                            sample.buttons = 0x1000;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_FILM_WIN=<ticks|lose|quit>[:A|:B]: each time a film challenge (the
                    // main challenge's ChallengeType Film, AnywhereFilm, SpotFilm, FriendFilm, PickaProFilm
                    // or TeamFilm) enters challenge_running, the local player's sequence begins; <ticks>
                    // later its active objectives complete as their completion scripts would (highlights
                    // included) and 30 ticks after that the sequence completes with 1500 points, which
                    // completes the group's points objective (lose: nothing completes, so the film's
                    // timer ends it as a loss; quit: 60 ticks in, cMsgAbandonChallenge as the start hub's
                    // Quit Challenge yes sends it, QuitChallengeCallback 0x8265e868). 60 ticks after the
                    // end-of-challenge options' screen is
                    // ready, the first one takes the given button (A, by default: Continue after a win,
                    // Retry after a loss; B: Retry after a win, Quit after a loss), a later one A; a Retry
                    // confirmation is answered Yes, then A goes to an unlock overlay. The challenge state
                    // and the career's front end are logged on each change.
                    if let Ok(film) = std::env::var("SKATE2_DIAGNOSTIC_FILM_WIN") {
                        use skate2_core::challenge::{Event, system::{Field, Message}};
                        static RUNNING: AtomicU64 = AtomicU64::new(u64::MAX);
                        static READY: AtomicU64 = AtomicU64::new(u64::MAX);
                        static DIALOG: AtomicU64 = AtomicU64::new(u64::MAX);
                        static SCREENS: AtomicU64 = AtomicU64::new(0);
                        static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
                        let (delay, button) = film.split_once(':').unwrap_or((film.as_str(), "A"));
                        let quit = delay.trim().eq_ignore_ascii_case("quit");
                        let delay = delay.trim().parse::<u64>().ok();
                        // Photos share the objective completion diagnostic, including team photo groups.
                        let film_type = matches!(self.session.audio_challenge().0, 6 | 7 | 37 | 38 | 39 | 40 | 41 | 46 | 47 | 48 | 49);
                        let state = self.session.career_main_challenge_state().filter(|_| film_type);
                        let summary = format!(
                            "state {state:?} nis_paused {} overlay {} end_ready {} dialog {}",
                            self.session.career_nis_paused(),
                            self.session.career_overlay_open(),
                            self.session.career_end_options_ready(),
                            self.session.career_dialog_open(),
                        );
                        if let Ok(mut last) = LAST.lock() && *last != summary {
                            eprintln!("diagnostic: film {summary} at {tick}");
                            *last = summary;
                        }
                        if state.as_deref() == Some("challenge_running") {
                            if RUNNING.load(Ordering::Relaxed) == u64::MAX {
                                RUNNING.store(tick, Ordering::Relaxed);
                                eprintln!("diagnostic: film sequence begins at {tick}");
                                self.session.diagnostic_post_challenge_event(Event::Challenge(Message::new(
                                    "cMsgSequenceBegin",
                                    vec![("PlayerID", Field::Number(0.0))],
                                )));
                            }
                            let running = RUNNING.load(Ordering::Relaxed);
                            if quit && running.checked_add(60) == Some(tick) {
                                eprintln!("diagnostic: film abandoned at {tick}");
                                self.session.diagnostic_post_challenge_event(Event::Challenge(Message::new(
                                    "cMsgAbandonChallenge",
                                    vec![("IsQuitting", Field::Bool(false))],
                                )));
                            }
                            if delay.and_then(|delay| running.checked_add(delay)) == Some(tick) {
                                eprintln!("diagnostic: film objectives {:?} completed at {tick}", self.session.diagnostic_complete_objectives());
                            }
                            if delay.and_then(|delay| running.checked_add(delay + 30)) == Some(tick) {
                                eprintln!("diagnostic: film sequence completes at {tick}");
                                self.session.diagnostic_post_challenge_event(Event::Challenge(Message::new(
                                    "cMsgSequenceComplete",
                                    vec![
                                        ("PlayerID", Field::Number(0.0)),
                                        ("SequenceScore", Field::Number(1500.0)),
                                        ("StartVisualGameTick", Field::Number(0.0)),
                                        ("StartTick", Field::Number(0.0)),
                                        ("TotalTrickCount", Field::Number(1.0)),
                                        ("StoredTrickCount", Field::Number(1.0)),
                                        ("Wipedout", Field::Bool(false)),
                                        ("UsedSessionMarker", Field::Bool(false)),
                                        ("StartSimGameTick", Field::Number(0.0)),
                                    ],
                                )));
                            }
                        } else {
                            RUNNING.store(u64::MAX, Ordering::Relaxed);
                        }
                        if self.session.career_end_options_ready() {
                            let ready = match READY.load(Ordering::Relaxed) { u64::MAX => { READY.store(tick, Ordering::Relaxed); tick } ready => ready };
                            if tick == ready + 60 {
                                let first = SCREENS.fetch_add(1, Ordering::Relaxed) == 0;
                                let retry = first && button.trim().eq_ignore_ascii_case("B");
                                sample.buttons = if retry { 0x2000 } else { 0x1000 };
                                eprintln!("diagnostic: film end options {} at {tick}", if retry { "B" } else { "A" });
                            }
                        } else {
                            READY.store(u64::MAX, Ordering::Relaxed);
                        }
                        if self.session.career_dialog_open() && SCREENS.load(Ordering::Relaxed) > 0 {
                            let opened = match DIALOG.load(Ordering::Relaxed) { u64::MAX => { DIALOG.store(tick, Ordering::Relaxed); tick } opened => opened };
                            if tick == opened + 30 { sample.buttons = 0x0002; }
                            if tick == opened + 60 {
                                sample.buttons = 0x1000;
                                eprintln!("diagnostic: film confirmation Yes at {tick}");
                            }
                        } else {
                            DIALOG.store(u64::MAX, Ordering::Relaxed);
                        }
                        if tick % 90 == 45 && self.session.career_overlay_open() && !self.session.career_end_options_ready()
                            && SCREENS.load(Ordering::Relaxed) > 0
                        {
                            eprintln!("diagnostic: film overlay A at {tick}");
                            sample.buttons = 0x1000;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_RACE=<win|lose|quit>[:A|:B]: while the main challenge is in
                    // run_heat, win teleports the local player to its next split-time gate every 90
                    // ticks (the trigger tester enters it there), quit sends cMsgAbandonChallenge 120
                    // ticks in (as the start hub's Quit Challenge yes, QuitChallengeCallback
                    // 0x8265e868) and lose leaves the race to the AI and the time limit. 60 ticks after
                    // the end-of-challenge options' screen is ready, the first one takes the given button
                    // (A by default; B: Retry after a win, Quit after a loss), later ones A; a Retry
                    // confirmation is answered Yes and A goes to an unlock overlay. The challenge state
                    // and the career's front end are logged on each change.
                    if let Ok(race) = std::env::var("SKATE2_DIAGNOSTIC_RACE") {
                        use skate2_core::challenge::{Event, system::{Field, Message}};
                        static PAGE_STEP: AtomicU64 = AtomicU64::new(0);
                        static RUNNING: AtomicU64 = AtomicU64::new(u64::MAX);
                        static READY: AtomicU64 = AtomicU64::new(u64::MAX);
                        static DIALOG: AtomicU64 = AtomicU64::new(u64::MAX);
                        static SCREENS: AtomicU64 = AtomicU64::new(0);
                        static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
                        let (mode, button) = race.split_once(':').unwrap_or((race.as_str(), "A"));
                        let state = self.session.career_main_challenge_state();
                        let summary = format!(
                            "state {state:?} nis_paused {} overlay {} end_ready {} dialog {}",
                            self.session.career_nis_paused(),
                            self.session.career_overlay_open(),
                            self.session.career_end_options_ready(),
                            self.session.career_dialog_open(),
                        );
                        if let Ok(mut last) = LAST.lock() && *last != summary {
                            eprintln!("diagnostic: race {summary} at {tick}");
                            *last = summary;
                        }
                        if state.as_deref() == Some("run_heat") {
                            let running = match RUNNING.load(Ordering::Relaxed) { u64::MAX => { RUNNING.store(tick, Ordering::Relaxed); tick } running => running };
                            let elapsed = tick - running;
                            if mode == "win" && elapsed > 0 && elapsed % 90 == 0 && let Some(gate) = self.session.diagnostic_race_next_gate() {
                                eprintln!("diagnostic: race gate {gate:?} at {tick}: {}", self.session.diagnostic_teleport_local_player(gate));
                            }
                            if mode == "quit" && elapsed == 120 {
                                eprintln!("diagnostic: race abandoned at {tick}");
                                self.session.diagnostic_post_challenge_event(Event::Challenge(Message::new(
                                    "cMsgAbandonChallenge",
                                    vec![("IsQuitting", Field::Bool(false))],
                                )));
                            }
                        } else {
                            RUNNING.store(u64::MAX, Ordering::Relaxed);
                        }
                        if self.session.career_end_options_ready() {
                            let ready = match READY.load(Ordering::Relaxed) { u64::MAX => { READY.store(tick, Ordering::Relaxed); tick } ready => ready };
                            let pages = mode == "pages";
                            if (!pages && tick == ready + 60) || (pages && tick > ready && (tick - ready) % 180 == 0) {
                                let first = SCREENS.fetch_add(1, Ordering::Relaxed) == 0;
                                let mut second = first && button.trim().eq_ignore_ascii_case("B");
                                if pages {
                                    let step = PAGE_STEP.fetch_add(1, Ordering::Relaxed);
                                    let options = self.session.diagnostic_end_options().unwrap_or([12; 3]);
                                    let slot = if step == 1 {
                                        options.iter().position(|&o| o == 14).unwrap_or(0)
                                    } else if (step >= 3 || !options.contains(&14)) && !options.contains(&13) && !options.contains(&0) {
                                        options.iter().position(|&o| o == 8).unwrap_or(0)
                                    } else { 0 };
                                    sample.buttons = [0x1000, 0x2000, 0x4000][slot];
                                    second = slot == 1;
                                    eprintln!("diagnostic: contest page step {step} options {options:?} button slot {slot} at {tick}");
                                } else {
                                    sample.buttons = if second { 0x2000 } else { 0x1000 };
                                }
                                eprintln!("diagnostic: race end options {} at {tick}", if second { "B" } else { "A" });
                            }
                        } else {
                            READY.store(u64::MAX, Ordering::Relaxed);
                            PAGE_STEP.store(0, Ordering::Relaxed);
                        }
                        if self.session.career_dialog_open() && SCREENS.load(Ordering::Relaxed) > 0 {
                            let opened = match DIALOG.load(Ordering::Relaxed) { u64::MAX => { DIALOG.store(tick, Ordering::Relaxed); tick } opened => opened };
                            if tick == opened + 30 { sample.buttons = 0x0002; }
                            if tick == opened + 60 {
                                sample.buttons = 0x1000;
                                eprintln!("diagnostic: race confirmation Yes at {tick}");
                            }
                        } else {
                            DIALOG.store(u64::MAX, Ordering::Relaxed);
                        }
                        if tick % 90 == 45 && self.session.career_overlay_open() && !self.session.career_end_options_ready()
                            && SCREENS.load(Ordering::Relaxed) > 0
                        {
                            eprintln!("diagnostic: race overlay A at {tick}");
                            sample.buttons = 0x1000;
                        }
                    }
                    // Scoped controller replay for career S.K.A.T.E. Each comma-separated
                    // value is the pop tick within one human attempt; the last value repeats.
                    // Only pad input changes: triggers, scoring and the original Lua decide
                    // whether the trick counts and when the next turn begins.
                    if let Ok(pops) = std::env::var("SKATE2_DIAGNOSTIC_SKATE_ATTEMPT") {
                        static ATTEMPT: std::sync::Mutex<(bool, u64, usize)> = std::sync::Mutex::new((false, 0, 0));
                        let state = self.session.career_main_challenge_state();
                        let playing = self.session.career_local_player() == Some(0)
                            && matches!(state.as_deref(), Some("wait_for_leader" | "wait_for_scoring" | "wait_for_follower" | "wait_for_copy"));
                        if let Ok(mut attempt) = ATTEMPT.lock() {
                            if playing && !attempt.0 {
                                attempt.1 = tick;
                                attempt.2 += 1;
                                eprintln!("diagnostic: skate human attempt {} at {tick}", attempt.2);
                            }
                            attempt.0 = playing;
                            if playing {
                                let elapsed = tick - attempt.1;
                                let values: Vec<u64> = pops.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                                if let Some(&pop) = values.get(attempt.2 - 1).or(values.last()) {
                                    sample.buttons = if elapsed < 100 { 0x1000 } else { 0 };
                                    if elapsed >= pop.saturating_sub(30) && elapsed < pop {
                                        sample.right_y = i16::MIN;
                                    } else if elapsed >= pop && elapsed < pop + 5 {
                                        sample.right_y = i16::MAX;
                                    }
                                    if let Ok(steer) = std::env::var("SKATE2_DIAGNOSTIC_SKATE_STEER") {
                                        let values: Vec<i64> = steer.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                                        if let [from, to, x] = values.as_slice() {
                                            if (elapsed as i64) >= *from && (elapsed as i64) < *to {
                                                sample.left_x = *x as i16;
                                            }
                                        }
                                    }
                                }
                                if elapsed % 30 == 0 {
                                    eprintln!("diagnostic: skate pad at {tick} elapsed={elapsed} state={state:?} board={:?} physics={:?}",
                                        self.session.diagnostic_board_kinematics(), self.session.diagnostic_state_ids());
                                }
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_MOVIE_SKIP: A every given number of ticks while FE state 28 holds
                    // the front end, counted from when it opened.
                    if let Some(every)=std::env::var("SKATE2_DIAGNOSTIC_MOVIE_SKIP").ok().and_then(|ticks|ticks.parse::<u64>().ok()).filter(|&every|every>0) {
                        static OPENED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
                        if self.session.movie_state_open() {
                            let opened = match OPENED.load(Ordering::Relaxed) { u64::MAX => { OPENED.store(tick, Ordering::Relaxed); tick } opened => opened };
                            if tick>opened && (tick-opened)%every==0 {
                                eprintln!("diagnostic: movie A at {} ticks", tick-opened);
                                sample.buttons=0x1000;
                            }
                        } else {
                            OPENED.store(u64::MAX, Ordering::Relaxed);
                        }
                    }
                    // Quit the career from its pause menu at the given tick, then the main menu's free skate.
                    if let Some(quit)=std::env::var("SKATE2_DIAGNOSTIC_CAREER_QUIT").ok().and_then(|tick|tick.parse::<u64>().ok())
                        && tick>=quit {
                        use skate2_core::ui::main_menu::Screen;
                        let pause=self.session.pause_menu_snapshot();
                        if tick==quit {
                            sample.buttons=0x0010;
                        } else if tick%30==0 && pause.visible {
                            sample.buttons=match pause.popup {
                                Some((_, selected))=>if selected==1 {0x1000} else {0x0002},
                                None if pause.options()[pause.selected]==skate2_core::ui::pause_menu::PauseMenuOption::Quit=>0x1000,
                                None=>0x0001,
                            };
                        } else if tick%30==0 && let Some(frame)=self.session.main_menu_frame()? {
                            match frame.screen {
                                Screen::MainMenu if frame.menu_visible => sample.buttons=if frame.selected==2 {0x1000} else {0x0002},
                                Screen::FreeSkate if frame.freeskate.as_ref().is_some_and(|f|f.image.is_some()) => sample.buttons=0x1000,
                                _ => {}
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_BUY_PROPERTY=<tick>[;<tick>...]: from each tick the menus buy the
                    // Stuff menu's first Property (buy_property_step), one press every 40 ticks.
                    if let Ok(starts)=std::env::var("SKATE2_DIAGNOSTIC_BUY_PROPERTY") {
                        static BEGUN: AtomicU64 = AtomicU64::new(0);
                        let begun = BEGUN.load(Ordering::Relaxed);
                        let next = starts.split(';').filter_map(|start| start.trim().parse::<u64>().ok()).nth(usize::try_from(begun).unwrap_or(usize::MAX));
                        if next == Some(tick) {
                            BEGUN.store(begun + 1, Ordering::Relaxed);
                            BUY_PHASE.store(1, Ordering::Relaxed);
                            eprintln!("diagnostic: buy property attempt {} at {tick}", begun + 1);
                        }
                        let busy = self.session.movie_state_open() || self.session.career_dialog_open() || self.session.career_overlay_open();
                        if tick % 40 == 0 && !busy
                            && let Some(buttons) = buy_property_step(&self.session.map_menu_snapshot(), self.session.pause_menu_snapshot())
                        {
                            sample.buttons = buttons;
                        }
                    }
                }
                if matches!(demo,ControlsDemo::MainMenu) {
                    sample.buttons=match tick {360|390=>0x0002,420|450=>0x0001,_=>0};
                    // SKATE2_DIAGNOSTIC_FRONTEND_BUTTONS=<tick>:<hex buttons>[;...]: these presses in
                    // place of the demo's own, until SKATE2_DIAGNOSTIC_FRONTEND_END ticks.
                    if let Ok(script)=std::env::var("SKATE2_DIAGNOSTIC_FRONTEND_BUTTONS") {
                        sample.buttons=script.split(';').find_map(|entry|{
                            let (at,buttons)=entry.split_once(':')?;
                            (at.trim().parse::<u64>().ok()?==tick).then(||u16::from_str_radix(buttons.trim(),16).ok())?
                        }).unwrap_or(0);
                    }
                }
                if matches!(demo,ControlsDemo::MainMenuGameplay) && tick==600 { sample.buttons=0x1000; }
                // SKATE2_DIAGNOSTIC_TURN_TRICK=<push ticks>:<flick tick>[:<step>[:<flicks>[:<volume>]]]:
                // each time an online challenge unpins the local skater, A held for the push ticks,
                // then a flick at the flick tick after the unpin, later by the step each turn, or with
                // a volume (a trigger's local id, hex) that many ticks after the skater enters it. The
                // flicks, one per turn in turn and `/` apart, are right-stick paths of compass points
                // such as `dr-d-ul`: the first held up to 30 ticks, the others 2, the last 4. The
                // default, `d-ul`, is a kickflip.
                if let (Ok(script), Some((unpinned, turn)))=(std::env::var("SKATE2_DIAGNOSTIC_TURN_TRICK"), self.session.diagnostic_online_turn()) {
                    let fields:Vec<&str>=script.split(':').map(str::trim).collect();
                    let number=|index:usize,default:u32|fields.get(index).and_then(|value|value.parse::<u32>().ok()).unwrap_or(default);
                    let push=number(0,60);
                    let flick=number(1,240)+number(2,0)*turn.saturating_sub(1);
                    if unpinned<push {sample.buttons|=0x1000;}
                    let volume=fields.get(4).and_then(|value|u64::from_str_radix(value.trim_start_matches("0x"),16).ok());
                    let ticks=match volume {
                        Some(volume)=>self.session.diagnostic_online_entered(volume),
                        None=>Some(unpinned),
                    };
                    let windup=flick.min(30);
                    let flicks:Vec<&str>=fields.get(3).filter(|flicks|!flicks.is_empty()).map_or_else(||vec!["d-ul"],|flicks|flicks.split('/').collect());
                    let path:Vec<&str>=flicks[usize::try_from(turn.saturating_sub(1)).unwrap_or(0)%flicks.len()].split('-').collect();
                    let middle=2*u32::try_from(path.len().saturating_sub(2)).unwrap_or(0);
                    let point=ticks.and_then(|ticks|if (flick-windup..flick).contains(&ticks) {path.first()} else {
                        ticks.checked_sub(flick).and_then(|after|if after<middle {path.get(1+usize::try_from(after/2).unwrap_or(0))} else if after<middle+4 {path.last()} else {None})
                    });
                    if let Some(point)=point {
                        sample.right_x=if point.contains('l') {i16::MIN} else if point.contains('r') {i16::MAX} else {0};
                        sample.right_y=if point.contains('u') {i16::MAX} else if point.contains('d') {i16::MIN} else {0};
                    }
                }
                // SKATE2_DIAGNOSTIC_DUMP_TRIGGERS=<radius>: the trigger volumes around the local skater
                // as an online challenge first unpins it.
                if let (Some(radius), Some((0, 1)))=(std::env::var("SKATE2_DIAGNOSTIC_DUMP_TRIGGERS").ok().and_then(|value|value.trim().parse::<f32>().ok()), self.session.diagnostic_online_turn()) {
                    if let Err(e)=self.session.diagnostic_dump_triggers(radius) {eprintln!("diagnostic triggers: {e}");}
                }
                // SKATE2_DIAGNOSTIC_PINNED_GESTURE=<period>: while an online challenge pins the local
                // skater, D-pad up for one tick every period ticks.
                if let Some(period)=std::env::var("SKATE2_DIAGNOSTIC_PINNED_GESTURE").ok().and_then(|value|value.trim().parse::<u64>().ok()).filter(|&period|period>0)
                    && self.session.diagnostic_online_pinned() && self.completed_frames%period==0 {
                    sample.buttons|=0x0001;
                }
                // SKATE2_DIAGNOSTIC_TRICK_CYCLE=<tick>[:<push ticks>:<flick tick>]: from that completed
                // tick, every five seconds a push, then a kickflip flick (right stick down for 30 ticks,
                // then up-left) at the flick tick.
                if let Ok(cycle)=std::env::var("SKATE2_DIAGNOSTIC_TRICK_CYCLE") {
                    let values:Vec<u64>=cycle.split(':').filter_map(|value|value.trim().parse().ok()).collect();
                    let (from,push,flick)=(values.first().copied().unwrap_or(u64::MAX),values.get(1).copied().unwrap_or(8),values.get(2).copied().unwrap_or(50).max(30));
                    if self.completed_frames>=from {
                        let phase=(self.completed_frames-from)%300;
                        if phase<push {sample.buttons|=0x1000;}
                        if (flick-30..flick).contains(&phase) {sample.right_y=i16::MIN;}
                        if (flick..flick+4).contains(&phase) {sample.right_x=i16::MIN;sample.right_y=i16::MAX;}
                    }
                }
                // Main menu row 3, three players join (the second picks the next skater), S.K.A.T.E.,
                // the first location (the snakes spot), then A to start.
                if matches!(demo,ControlsDemo::MainMenuFreeskate) && std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY").is_some() {
                    let back=std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY_BACK").is_some();
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_GAME picks Spot Battle ("spot") or Hall of Meat
                    // ("hom") instead of S.K.A.T.E.
                    let game=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_GAME").unwrap_or_default();
                    let game_presses: &[u64]=match game.as_str() { "spot"=>&[], "hom"=>&[1540,1570], _=>&[1540] };
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_CAREER keeps the first player on the first
                    // skater, the created one when the profile has it.
                    let career=std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY_CAREER").is_some();
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_DISTINCT: the third player steps two skaters on, so
                    // the two later players load different skaters.
                    let distinct=std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY_DISTINCT").is_some();
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_TWO: two players (no right on the player count, no
                    // third join), so a game of three rounds ends after six turns.
                    let two=std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY_TWO").is_some();
                    sample.buttons=match tick {
                        360|390|420=>0x0002,
                        1000 if back=>0x2000,
                        _ if back && tick>1000=>0,
                        1180 if career=>0,
                        1000|1420|1440|1460|1480 if two=>0,
                        1440|1460 if distinct=>0x0008,
                        450|1060|1120|1240|1300|1360|1420|1480|1600=>0x1000,
                        1000|1180=>0x0008,
                        _ if game_presses.contains(&tick)=>0x0008,
                        // In the game: A answers each turn's press-A dialog (A also pushes, which
                        // would roll the skater off the spot).
                        _ if tick>1800 && tick%60==0 && self.session.career_press_a_open()=>0x1000,
                        _=>0,
                    };
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_LOCATION=<text>: right through the locations until the
                    // location's title ID holds the text, then the start.
                    let wanted=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_LOCATION").ok().map(|text|text.to_ascii_uppercase());
                    if !back && !self.party_location_started && tick>=1760
                        && let Some(party)=self.session.main_menu_frame()?.and_then(|frame|frame.party_play) {
                        if wanted.as_ref().is_none_or(|text|party.location.to_ascii_uppercase().contains(text.as_str())) {
                            self.party_location_started=true;
                            sample.buttons=0x1000;
                        } else if tick%20==0 {
                            sample.buttons=0x0008;
                        }
                    }
                    // The first player sets a trick at the snakes spot (t_5b_01_skate_snakes): six
                    // pushes along spawn01's heading to 14 m, the stick wound up inside the first
                    // spot volume, and a pop near 17 m (varied by turn) over the ledge at about 18.8 m
                    // and down into the second volume, which starts about 23.4 m out. Each of its
                    // turns flicks another trick (an ollie, a nollie, both flips, a shove-it), so
                    // none was already done; the others stand still and their turns run out.
                    let open=self.session.career_press_a_open();
                    let first=self.session.career_local_player()==Some(0);
                    if open {
                        self.party_dialog_seen=true;
                    } else if std::mem::take(&mut self.party_dialog_seen) && first {
                        self.party_first_player_turns+=1;
                    }
                    if !back && !open && first && self.party_first_player_turns>0
                        && let Some((position,_))=self.session.map_player_pose() {
                        const SPAWN:[f32;2]=[915.8404,19.413025];
                        const HEADING:[f32;2]=[0.6298591,-0.7765197];
                        let along=(position[0]-SPAWN[0])*HEADING[0]+(position[1]-SPAWN[1])*HEADING[1];
                        let (wind,flick)=match (self.party_first_player_turns-1)%5 {
                            0=>((0,i16::MIN),(0,i16::MAX)),
                            1=>((0,i16::MAX),(0,i16::MIN)),
                            2=>((0,i16::MIN),(i16::MIN,i16::MAX)),
                            3=>((0,i16::MIN),(i16::MAX,i16::MAX)),
                            _=>((0,i16::MIN),(i16::MIN,0)),
                        };
                        let pop=[17.2,17.35,17.1,17.5,17.25][((self.party_first_player_turns-1)%5) as usize];
                        if tick%15==0 && let Some((position,_,speed))=self.session.diagnostic_board_kinematics() {
                            eprintln!("Party Play S.K.A.T.E. {tick}: turn {} pop {pop} along {along:.2} y {:.2} speed {speed:.2}",
                                self.party_first_player_turns, position[1]);
                        }
                        if along<14.0 {
                            if tick%20==0 { sample.buttons=0x1000; }
                        } else if along<pop {
                            (sample.right_x,sample.right_y)=wind;
                        } else if along<pop+1.0 {
                            (sample.right_x,sample.right_y)=flick;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_CHANGE: X on the results opens Change Game, which
                    // steps to S.K.A.T.E. and its second location, goes back, opens again, returns
                    // to the running game and restarts it with A; `view` stays on the screen after
                    // the steps, and `switch` picks S.K.A.T.E.'s first location with A.
                    if let Ok(change_mode)=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_CHANGE")
                        && !(change_mode=="view" && self.party_change_step>=4) {
                        let results=self.session.career_end_options_ready();
                        let change=self.session.career_change_game_ready();
                        self.party_change_wait=if results||change { self.party_change_wait+1 } else { 0 };
                        let wait=self.party_change_wait;
                        let switch=change_mode=="switch";
                        let press=match (self.party_change_step,results,change) {
                            (1,_,true) if switch && wait>=60=>Some(0x0008),
                            (2,_,true) if switch && wait>=90=>Some(0x1000),
                            _ if switch && self.party_change_step>0=>None,
                            (0|5,true,_) if wait>=120=>Some(0x4000),
                            (1|3,_,true) if wait>=60=>Some(0x0008),
                            (2,_,true) if wait>=60=>Some(0x0002),
                            (4,_,true) if wait>=90=>Some(0x2000),
                            (6,_,true) if wait>=60=>Some(0x0004),
                            (7,_,true) if wait>=90=>Some(0x1000),
                            _=>None,
                        };
                        if let Some(buttons)=press {
                            eprintln!("Party Play change {tick}: step {} presses {buttons:#06x}", self.party_change_step);
                            sample.buttons=buttons;
                            self.party_change_step+=1;
                            self.party_change_wait=0;
                        }
                    }
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT: Start at the given tick, Quit and its Yes,
                    // then the main menu's free skate with its first skater, who pushes off;
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_ROW=0 takes the career row instead.
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_PLAYER=<player> waits for that player's turn
                    // before Start, and SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_CHALLENGE=<challenge> signs
                    // the challenge up in the game the quit reaches, then A on its poster and dialogs.
                    if let Some(quit)=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT").ok().and_then(|tick|tick.parse::<u64>().ok())
                        && tick>=quit {
                        use skate2_core::ui::main_menu::Screen;
                        let row=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_ROW").ok().and_then(|row|row.parse::<usize>().ok()).unwrap_or(2);
                        (sample.buttons,sample.right_x,sample.right_y)=(0,0,0);
                        let pause=self.session.pause_menu_snapshot();
                        static QUIT_STARTED: AtomicBool=AtomicBool::new(false);
                        let player=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_PLAYER").ok().and_then(|player|player.parse::<i32>().ok());
                        let start=!QUIT_STARTED.load(Ordering::Relaxed)
                            && player.is_none_or(|player|self.session.career_local_player()==Some(player) && !self.session.career_press_a_open());
                        if start {
                            QUIT_STARTED.store(true,Ordering::Relaxed);
                            eprintln!("Party Play quit {tick}: Start on player {:?}'s turn", self.session.career_local_player());
                            sample.buttons=0x0010;
                        } else if !QUIT_STARTED.load(Ordering::Relaxed) {
                            if tick%60==0 && self.session.career_press_a_open() { sample.buttons=0x1000; }
                        } else if tick%30==0 && pause.visible {
                            sample.buttons=match pause.popup {
                                Some((_, selected))=>if selected==1 {0x1000} else {0x0002},
                                None if pause.options()[pause.selected]==skate2_core::ui::pause_menu::PauseMenuOption::Quit=>0x1000,
                                None=>0x0001,
                            };
                        } else if tick%30==0 && let Some(frame)=self.session.main_menu_frame()? {
                            self.party_quit_menu_seen=true;
                            match frame.screen {
                                Screen::MainMenu if frame.menu_visible => sample.buttons=if frame.selected==row {0x1000} else {0x0002},
                                Screen::FreeSkate if frame.freeskate.as_ref().is_some_and(|f|f.image.is_some()) => sample.buttons=0x1000,
                                _ => {}
                            }
                        } else if !self.party_quit_menu_seen && !pause.visible && self.session.career_local_player().is_some() {
                            if tick%60==0 && self.session.career_press_a_open() {
                                sample.buttons=0x1000;
                            } else if tick%60==15 && !self.session.career_press_a_open() {
                                sample.buttons=0x0010;
                            }
                        } else if self.party_quit_menu_seen && self.session.main_menu_frame()?.is_none() && !pause.visible {
                            if let Ok(name)=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_QUIT_CHALLENGE") {
                                static SIGNED: AtomicBool=AtomicBool::new(false);
                                if tick%60==0 && !SIGNED.load(Ordering::Relaxed) && self.session.diagnostic_sign_up_challenge(&name) {
                                    eprintln!("Party Play quit {tick}: signed up {name}");
                                    SIGNED.store(true,Ordering::Relaxed);
                                }
                                if tick%30==0 && (self.session.career_poster_ready() || self.session.career_press_a_open()) {
                                    sample.buttons=0x1000;
                                }
                                if tick%60==0 {
                                    eprintln!("Party Play quit {tick}: local {:?} players {:?}", self.session.career_local_player(), self.session.diagnostic_player_rows());
                                }
                            } else {
                                if tick%20==0 { sample.buttons=0x1000; }
                                if tick%100==0 && let Some((position,_))=self.session.map_player_pose() {
                                    eprintln!("Party Play quit {tick}: free skate at {position:?}");
                                }
                            }
                        }
                    }
                    // SKATE2_DIAGNOSTIC_PARTY_PLAY_SETTINGS=<tick>: from there Start, Game Settings, Audio
                    // Settings, SFX down to 80%, then B out of both pages, which saves the profile.
                    if let Some(at)=std::env::var("SKATE2_DIAGNOSTIC_PARTY_PLAY_SETTINGS").ok().and_then(|tick|tick.parse::<u64>().ok())
                        && tick>=at && !self.session.career_press_a_open() {
                        use skate2_core::ui::{options::Page, pause_menu::PauseMenuOption};
                        (sample.buttons,sample.right_x,sample.right_y)=(0,0,0);
                        let pause=self.session.pause_menu_snapshot();
                        let options=self.session.options_snapshot();
                        let lowered=self.session.profile_settings().sfx_volume<0.85;
                        if tick%30==0 {
                            sample.buttons=match options.page {
                                Page::Audio if options.volumes[0]>8=>0x0004,
                                Page::Audio=>0x2000,
                                Page::Options if lowered=>0x2000,
                                Page::Options=>if options.selected==1 {0x1000} else {0x0002},
                                _ if lowered=>0,
                                _ if !pause.visible=>0x0010,
                                _ if pause.popup.is_none()=>
                                    if pause.options()[pause.selected]==PauseMenuOption::Settings {0x1000} else {0x0002},
                                _=>0,
                            };
                        }
                        if tick%300==0 {
                            let store=skate2_core::edit_skater_profile::EditSkaterProfileStore::open(
                                skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?)?;
                            eprintln!("Party Play settings {tick}: page {:?} sfx {} saved sfx {}", options.page,
                                self.session.profile_settings().sfx_volume, store.load_settings()?.sfx_volume);
                        }
                    }
                    if tick%100==0 && let Some(frame)=self.session.main_menu_frame()? {
                        eprintln!("Party Play {tick}: {:?} {:?}", frame.screen, frame.party_play);
                    }
                } else if matches!(demo,ControlsDemo::MainMenuFreeskate) {
                    sample.buttons=match tick {
                        360|390=>0x0002,
                        420|1260=>0x1000,
                        900|990|1020=>0x0008,
                        960|1080=>0x0002,
                        1110|1140=>0x0004,
                        _=>0,
                    };
                    // SKATE2_DIAGNOSTIC_FREESKATE_LOCATION: the Location row steps right to the first
                    // location whose name contains it and starts there, then the skater pushes off.
                    if let Ok(target)=std::env::var("SKATE2_DIAGNOSTIC_FREESKATE_LOCATION") && tick>=990 {
                        sample.buttons=0;
                        if self.freeskate_location_entered {
                            if (1900..2400).contains(&tick) && tick%20==0 { sample.buttons=0x1000; }
                        } else if tick%20==0 && let Some(screen)=self.session.main_menu_frame()?.and_then(|frame|frame.freeskate) {
                            if !screen.location.to_lowercase().contains(&target.to_lowercase()) {
                                eprintln!("Free Skate location {tick}: {}", screen.location);
                                sample.buttons=0x0008;
                            } else if screen.image.is_some() {
                                eprintln!("Free Skate location {tick}: starting at {}", screen.location);
                                self.freeskate_location_entered=true;
                                sample.buttons=0x1000;
                            }
                        }
                    }
                    if tick==1250 {
                        let freeskate=self.session.main_menu_frame()?.and_then(|frame|frame.freeskate);
                        eprintln!("Free Skate: {freeskate:?}");
                    }
                    // Back instead: B at the screen, then the main menu after its NIS.
                    if std::env::var_os("SKATE2_DIAGNOSTIC_FREESKATE_BACK").is_some() {
                        if tick==1260 { sample.buttons=0x2000; }
                        if tick==1700 {
                            let frame=self.session.main_menu_frame()?;
                            eprintln!("Free Skate back: {:?}", frame.map(|frame|(frame.screen,frame.menu_visible,frame.selected)));
                        }
                    }
                    if tick==2500 {
                        let matrix=self.session.map_locations().iter()
                            .find(|location|location.localization_key=="ID_LOCATION_MATRIXPLAZA")
                            .map(|location|location.world_position);
                        eprintln!("Free Skate: skater at {:?}, Matrix Plaza at {matrix:?}", self.session.map_player_pose());
                    }
                    // StartHub -> Settings -> the music player (or Audio Settings), then each screen's input.
                    if let Ok(mode) = std::env::var("SKATE2_DIAGNOSTIC_MUSIC_PLAYER") && tick > 2500 && tick % 30 == 0 {
                        use skate2_core::ui::{music_player::AptState, options::Page, pause_menu::PauseMenuOption};
                        let pause = self.session.pause_menu_snapshot();
                        let options = self.session.options_snapshot();
                        let music = self.session.music_player_snapshot().clone();
                        // Save: the music player's mode, then SFX at 80%, then out of Options.
                        let saved = mode == "save" && self.session.profile_settings().sfx_volume < 0.85;
                        if tick == 2520 {
                            sample.buttons = 0x0010;
                        } else if music.visible {
                            match (mode.as_str(), music.state) {
                                // Transition: B with the screen up, then Options' Music Player row again.
                                ("transition", _) if tick % 120 == 0 => sample.buttons = 0x2000,
                                ("transition", _) => {}
                                (_, AptState::Ambient) => sample.right_trigger = 255,
                                ("save", AptState::Player) => sample.buttons = 0x2000,
                                ("edit", AptState::Player) => sample.buttons = 0x1000,
                                ("edit", AptState::Edit) if music.selected < 3 => sample.buttons = 0x0002,
                                ("edit", AptState::Edit) if music.edit_items[music.selected].is_some_and(|(_, on)| on) => sample.buttons = 0x4000,
                                ("next", AptState::Player) if music.playlist_number == 1 => sample.buttons = 0x0008,
                                _ => {}
                            }
                        } else if options.page == Page::Options && saved {
                            sample.buttons = 0x2000;
                        } else if options.page == Page::Options && mode == "exit" {
                            // Exit: B with Options up, then the StartHub's Game Settings again.
                            if tick % 240 == 0 { sample.buttons = 0x2000; }
                        } else if options.page == Page::Options {
                            let target = usize::from(mode == "audio" || (mode == "save" && self.session.music_mode() == 2));
                            sample.buttons = if options.selected == target { 0x1000 } else { 0x0002 };
                        } else if options.page == Page::Audio {
                            if options.volumes[0] > 8 { sample.buttons = 0x0004; } else if mode == "save" { sample.buttons = 0x2000; }
                        } else if saved {
                            let store = skate2_core::edit_skater_profile::EditSkaterProfileStore::open(
                                skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?)?;
                            if tick % 300 == 0 {
                                eprintln!("Music player diagnostic {tick}: saved sfx {} music mode {}",
                                    store.load_settings()?.sfx_volume, store.load_music_player(&[])?.music_mode);
                            }
                        } else if mode == "map" && self.session.map_menu_snapshot().visible {
                            // Map: B with the map up, then the StartHub's Map again.
                            if tick % 240 == 0 { sample.buttons = 0x2000; }
                        } else if pause.visible && pause.popup.is_none() {
                            let row = if mode == "map" { PauseMenuOption::Map } else { PauseMenuOption::Settings };
                            sample.buttons = if pause.options()[pause.selected] == row { 0x1000 } else { 0x0002 };
                        }
                        if tick % 300 == 0 {
                            eprintln!("Music player diagnostic {tick}: options {options:?}");
                            eprintln!("Music player diagnostic {tick}: {music:?} mode {} playlist {:?}",
                                self.session.music_mode(), self.session.music_player_playlist());
                        }
                        if matches!(mode.as_str(), "transition" | "exit" | "map") && sample.buttons & 0x3000 != 0 {
                            eprintln!("Music player transition {tick}: buttons {:#06x} at {:.3} s", sample.buttons,
                                skate2_core::diagnostics::start().elapsed().as_secs_f64());
                        }
                    }
                    // Quit from the free skate pause menu, then a second free skate with the defaults.
                    if std::env::var_os("SKATE2_DIAGNOSTIC_FREESKATE_QUIT").is_some() && tick>2500 {
                        use skate2_core::ui::main_menu::Screen;
                        let pause=self.session.pause_menu_snapshot();
                        if tick==2520 {
                            eprintln!("Free Skate pause options: {:?}", pause.options());
                            sample.buttons=0x0010;
                        }
                        if tick%30==0 && pause.visible {
                            sample.buttons=match pause.popup {
                                Some((_, selected))=>if selected==1 {0x1000} else {0x0002},
                                None if pause.options()[pause.selected]==skate2_core::ui::pause_menu::PauseMenuOption::Quit=>0x1000,
                                None=>0x0001,
                            };
                        }
                        if tick%30==0 && !pause.visible {
                            if let Some(frame)=self.session.main_menu_frame()? {
                                match frame.screen {
                                    Screen::MainMenu if frame.menu_visible => sample.buttons=if frame.selected==2 {0x1000} else {0x0002},
                                    Screen::FreeSkate if frame.freeskate.as_ref().is_some_and(|f|f.image.is_some()) => {
                                        eprintln!("Free Skate again: {:?}", frame.freeskate);
                                        sample.buttons=0x1000;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if tick==6400 {
                            let downtown=self.session.map_locations().iter()
                                .find(|location|location.localization_key=="ID_LOCATION_DOWNTOWN")
                                .map(|location|location.world_position);
                            eprintln!("Free Skate again: skater at {:?}, Downtown at {downtown:?}", self.session.map_player_pose());
                        }
                    }
                }
                if matches!(demo,ControlsDemo::MainMenuRoundtrip){
                    if tick==600{sample.buttons=0x1000;}
                    if tick==850{sample.buttons=0x0010;}
                    let pause=self.session.pause_menu_snapshot();
                    if (900..1300).contains(&tick)&&tick%30==0&&pause.visible{
                        sample.buttons=if let Some((_, selected))=pause.popup {if selected==1{0x1000}else{0x0002}}else if pause.options()[pause.selected]==skate2_core::ui::pause_menu::PauseMenuOption::Quit{0x1000}else{0x0002};
                    }
                }
                if matches!(demo,ControlsDemo::DisplaySettings) {
                    // Only ordinary raw PRES input; no controller or save injection.
                    sample.buttons=match tick {
                        360|390|420|450|480 => 0x0002, // main Options
                        540|600 => 0x1000, // Options -> Display Settings
                        570|585|1300|1310|1316 => 0x0002, // past Audio and Video Settings (and in game the Music Player)
                        750|960|990|1380 => 0x0002,
                        840|870 => 0x2000, // Back, Back -> native save boundary
                        1020|1290|1320 => 0x1000,
                        1170 => 0x0010,
                        1230|1260 => 0x0001, // paused Debug -> Quit -> Settings
                        _=>0,
                    };
                    let display=self.session.options_snapshot();
                    if tick==0 { eprintln!("Display Settings: loaded profile preferences {:?}", display.values); }
                    if tick==690 && display.values.postfx {sample.buttons=0x0008;}
                    if tick==780 && display.values.fisheye {sample.buttons=0x0008;}
                    if tick==800 {sample.buttons=0x0002;} // Frame Rate Cap
                    let capped=display.values.frame_rate_cap==skate2_core::ui::options::FrameRateCap::Fps60;
                    if matches!(tick,810|820|830) && !capped {sample.buttons=0x0004;}
                    if tick==1350 { sample.buttons=0x0008; } // immediate paused exposure on
                    if tick==1365 { sample.buttons=0x0008; } // and off
                    if tick==1410 { sample.buttons=0x0004; } // fisheye on
                    if tick==1440 { sample.buttons=0x0008; } // and off
                    if tick==900 {
                        let store=skate2_core::edit_skater_profile::EditSkaterProfileStore::open(
                            skate2_core::edit_skater_profile::EditSkaterProfileStore::default_path()?)?;
                        let expected=skate2_core::ui::options::PcDisplaySettings{postfx:false,fisheye:false,
                            frame_rate_cap:skate2_core::ui::options::FrameRateCap::Fps60,fps_counter:false,fullscreen:false,
                            resolution_scale:100,vsync:true};
                        if display.page!=skate2_core::ui::options::Page::Hidden
                            || display.values!=expected || store.load_display_settings()?!=expected {
                            return Err("Display Settings main-menu save roundtrip failed".into());
                        }
                        eprintln!("Display Settings: main-menu exit saved both disabled and the 60 cap to SKATER.P");
                    }
                }
                self.frontend_demo_ticks+=1;
            }
            if matches!(demo, ControlsDemo::EditSkater | ControlsDemo::EditSkaterEyes | ControlsDemo::EditSkaterRoundtrip | ControlsDemo::EditSkaterCouch | ControlsDemo::EditSkaterStyles | ControlsDemo::EditSkaterGender | ControlsDemo::EditSkaterPopup | ControlsDemo::EditSkaterRemoveShirt) {
                // Diagnostic input clock advances while gameplay is paused.
                // Enter through StartHub; never force the editor state.
                let tick = self.frontend_demo_ticks;
                if tick == 60 { sample.buttons = 0x0010; }
                let pause = self.session.pause_menu_snapshot();
                if (120..600).contains(&tick) && tick % 30 == 0 && pause.visible
                    && self.session.edit_skater_view()?.is_none() {
                    sample.buttons = if pause.options()[pause.selected] == skate2_core::ui::pause_menu::PauseMenuOption::EditSkater { 0x1000 } else { 0x0002 };
                }
                if matches!(demo, ControlsDemo::EditSkaterCouch | ControlsDemo::EditSkaterStyles | ControlsDemo::EditSkaterRemoveShirt) {
                    if tick == 660 { sample.buttons = 0x0002; }
                    if tick == 690 { sample.buttons = 0x1000; }
                }
                if matches!(demo, ControlsDemo::EditSkaterStyles) && tick >= 750 && tick % 30 == 0 {
                    use skate2_core::ui::edit_skater::EditSkaterMenu as M;
                    if let Some(view) = self.session.edit_skater_view()? {
                        match view.menu {
                            M::Slappys => sample.buttons = 0x1000,
                            M::PartThumbnails => sample.buttons = if tick % 60 == 0 { 0x0008 } else { 0x1000 },
                            M::WearStyles => {
                                if let Some(style) = view.rows.iter().position(|r|r.kind == "category") {
                                    if view.selected != style { sample.buttons = 0x0002; }
                                    else if tick == 1200 { sample.buttons = 0x0008; }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if matches!(demo, ControlsDemo::EditSkaterEyes) && tick >= 660 && tick % 30 == 0 {
                    use skate2_core::ui::edit_skater::EditSkaterMenu as M;
                    if let Some(view) = self.session.edit_skater_view()? {
                        sample.buttons = match view.menu {
                            M::EditSkaterMain | M::FaceMod => 0x1000,
                            M::BackAlleyPete => if view.selected + 1 == view.rows.len() {0x1000} else {0x0002},
                            M::Eyes if tick <= 1230 => {
                                if std::env::var_os("SKATE2_DIAGNOSTIC_LAST_EYE").is_some() {0x0008} else {0x0004}
                            }
                            _ => 0,
                        };
                    }
                }
                if matches!(demo, ControlsDemo::EditSkaterRoundtrip) && tick == 900 {
                    sample.buttons = 0x0010;
                }
                // Finish's exit confirmation, answered with its default Yes.
                if tick % 30 == 15 && self.session.edit_skater_view()?.is_some_and(|v| v.exit_confirmation.is_some()) {
                    sample.buttons = 0x1000;
                }
                if matches!(demo, ControlsDemo::EditSkaterRemoveShirt) {
                    // Slappy's Hats -> T-Shirts -> Remove. Inspect the
                    // actual recipe afterwards, not only lack of errors.
                    if tick == 750 { sample.buttons = 0x0002; }
                    if matches!(tick, 810 | 900) { sample.buttons = 0x1000; }
                }
                if matches!(demo, ControlsDemo::EditSkaterGender | ControlsDemo::EditSkaterPopup)
                    && matches!(tick, 660 | 720) {
                    sample.buttons = 0x1000; // Body Mods, then Gender.
                }
                if matches!(demo, ControlsDemo::EditSkaterGender)
                    && matches!(tick, 900 | 1200 | 1290) {
                    sample.buttons = 0x1000; // Accept female, reopen, restore male.
                }
                self.frontend_demo_ticks += 1;
            }
            if matches!(demo, ControlsDemo::MapTeleport) {
                let tick = self.frontend_demo_ticks;
                if tick == 60 { sample.buttons = 0x0010; }
                let pause = self.session.pause_menu_snapshot();
                let map = self.session.map_menu_snapshot();
                if (120..600).contains(&tick) && tick % 30 == 0 && pause.visible && !map.visible {
                    sample.buttons = if pause.options()[pause.selected] == skate2_core::ui::pause_menu::PauseMenuOption::Map { 0x1000 } else { 0x0002 };
                }
                if map.visible && (600..=960).contains(&tick) && tick % 90 == 0 {
                    sample.buttons = 0x1000; // Locations, Landmarks, location, Teleport.
                }
                self.frontend_demo_ticks += 1;
            }
            if matches!(demo, ControlsDemo::PedestrianApproach) && self.completed_frames >= 3000 {
                [sample.left_x, sample.left_y] = self.pedestrian_approach_stick;
            }
            if let Some(probe)=self.replay_probe.as_mut() {sample=probe.input(&self.session,self.completed_frames)?;}
            self.pad.demo_sample = Some(sample);
        } else if let Some(hands) = self.ground_grab {
            let mut sample = neutral_demo_sample();
            if (32..272).contains(&self.completed_frames) {
                if hands & 1 != 0 {
                    sample.left_trigger = u8::MAX;
                }
                if hands & 2 != 0 {
                    sample.right_trigger = u8::MAX;
                }
            }
            self.pad.demo_sample = Some(sample);
        } else if self.manual_demo {
            let mut sample = if (32..140).contains(&self.completed_frames) {
                demo_push_sample()
            } else {
                neutral_demo_sample()
            };
            if (160..400).contains(&self.completed_frames) {
                sample.right_y = -18000;
            }
            self.pad.demo_sample = Some(sample);
        } else if self.y_demo {
            self.pad.demo_sample = Some(demo_y_sample(self.completed_frames == 32));
        } else if self.push_demo {
            // The corpus boots with one neutral completed frame, then
            // indexes its 120-tick replay from step zero.
            let corpus_step = self.completed_frames.checked_sub(1);
            self.pad.demo_sample = Some(
                if corpus_step.is_some_and(|step| (32..44).contains(&step)) {
                    demo_push_sample()
                } else {
                    neutral_demo_sample()
                },
            );
        }
        if let Some(fuzz) = self.input_fuzz.as_mut() {
            self.pad.demo_sample = Some(fuzz.sample(self.completed_frames));
        }
        let was_paused = self.session.pause_menu_snapshot().visible;
        self.session.sync_presentation_navigation(navigation)?;
        Ok(was_paused)
    }

    fn run_tick(&mut self, was_paused: bool) -> TickRecord {
        let tick_start = Instant::now();
        skate2_core::diagnostics::lean_trace::begin_tick();
        let replay_or_window_focus = self.pad.demo_sample.is_some() || self.focused;
        let outcome = self
            .session
            .try_tick(
                &mut self.pad,
                SelectedQueuedFramePolicy {
                    dt: 1.0 / 60.0,
                    lod: 0,
                    wait_for_bs_job: true,
                    can_simulate: true,
                    // An explicit replay supplies a virtual focused pad;
                    // tools/window activation must not discard its Y/stick
                    // samples. Real XInput keeps the actual window focus.
                    has_focus: replay_or_window_focus,
                    ground_group: 0,
                    allow_multiple_writes: true,
                    spy_flags: 4,
                    numerics: SelectedFrameNumerics::ExistingHostApproximation,
                    physics_flags3: true,
                    park_grind_preparation: true,
                    exclude_grind_fill: true,
                },
                &mut || Err::<i32, _>("hands clamp service has no first-test host answer"),
            )
            .map_err(|error| {
                format!(
                    "controlled tick after {} completed frames: {error}",
                    self.completed_frames
                )
            });
        let tick_time = tick_start.elapsed();
        TickRecord {
            due: self.previous.checked_sub(self.accumulator).unwrap_or(tick_start),
            started: tick_start,
            was_paused,
            outcome,
            tick_time,
            core: if skate2_core::diagnostics::frame_profile::enabled() {
                skate2_core::diagnostics::frame_profile::take()
            } else {
                [Duration::ZERO; skate2_core::diagnostics::frame_profile::COUNT]
            },
        }
    }

    /// Host consumption of one tick: display settings, visual snapshot and
    /// view upload, acknowledgements and diagnostics. True ends the redraw's
    /// tick loop (pause boundary or completion bound).
    fn complete_tick(
        &mut self,
        view: &mut LiveSkatingView,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        record: TickRecord,
        mut restorer: Option<&mut PresentationRestorer>,
    ) -> RenderResult<bool> {
        let TickRecord { due, started, was_paused, outcome, tick_time, core } = record;
        if let Some(trace) = &mut self.tick_trace {
            trace.push([due, started, started + tick_time, Instant::now()]);
        }
        let outcome = outcome?;
        self.gameplay = !matches!(outcome, SelectedQueuedFrameOutcome::Frontend);
        let ended_career_hud = self.session.take_ended_career_hud();
        if self.session.take_game_unloaded() {
            view.exit_to_front_end()?;
        } else if let Some(last) = ended_career_hud {
            view.end_career(&last)?;
        }
        for (total, value) in self.frame_timings.core.iter_mut().zip(core) {
            *total += value;
        }
        let options_menu = self.session.options_snapshot();
        renderer.set_display_settings(options_menu.values);
        renderer.set_cutscene_postfx(self.session.cutscene_postfx_active());
        view.set_options_menu(options_menu)?;
        view.set_free_skate_settings(self.session.free_skate_hud_settings());
        view.set_music_player(self.session.music_player_snapshot())?;
        view.set_skate_profile(self.session.skate_profile_view())?;
        if self.session.edit_skater_view()?.is_none() {
            view.close_editor(scene);
        }
        if self.session.trick_guide_snapshot().is_none(){view.close_trick_guide(scene);}
        if self.session.main_menu_frame()?.is_none(){view.close_main_menu(scene)?;}
        let (audio_state,audio_world)=self.audio_inputs();
        let menu_sounds = self.session.take_menu_sounds();
        view.play_nis_sfx(self.session.take_nis_sfx());
        let music = self.session.take_music_requests();
        let mut chyron = self.session.take_chyron_songs();
        if let Some((song, index)) =
            view.update_audio(FIXED_STEP.as_secs_f32(), audio_state, audio_world, menu_sounds, self.session.music_player_playlist(), music)
        {
            self.session.set_music_player_song(song, index);
        }
        chyron.extend(view.take_audio_chyron());
        view.update_chyron(self.session.chyron_state(), &chyron)?;
        view.update_online_event_log(self.session.online_hud_active(), self.session.take_online_event_log())?;
        view.update_online_chat(self.session.online_chat_prompt().as_ref(), self.session.online_nickname_prompt().as_ref())?;
        let (lobby_states, lobby_title) = self.session.take_online_lobby_hud();
        view.update_online_lobby_hud(&lobby_states, &lobby_title)?;
        let (hub_opened, hub_calls) = self.session.take_online_hub_calls();
        let hub = self.session.online_hub_snapshot();
        let hub_screen = self.session.online_hub_match_snapshot();
        let hub_popup = self.session.online_hub_popup();
        let central = self.session.take_online_central_messages();
        view.update_online_hub(renderer, hub.as_ref(), hub_opened, &hub_calls, hub_screen.as_ref(), hub_popup.as_ref(), &self.session.online_nickname(), &central)?;
        if view.take_online_hub_intro() {
            self.session.online_hub_ready();
        }
        if hub_screen.is_some() && view.take_online_match_intro() {
            self.session.online_hub_screen_ready();
        }
        if skate2_core::diagnostics::frame_profile::enabled() {
            for (total, value) in self
                .frame_timings
                .core
                .iter_mut()
                .zip(skate2_core::diagnostics::frame_profile::take())
            {
                *total += value;
            }
        }
        self.frame_timings.ticks += 1;
        self.frame_timings.tick += tick_time;
        match outcome {
            SelectedQueuedFrameOutcome::Frontend => {
                let pause = self.session.pause_menu_snapshot();
                // A pause boundary discards only host timer debt. No paused
                // wall time reaches physics after resume, and the close
                // button never enters the gameplay command queue.
                if was_paused != pause.visible {
                    if !pause.visible {
                        let career = self.session.take_career_hud();
                        view.update_pause(renderer, scene, now, &pause,
                            &self.session.map_menu_snapshot(), self.session.map_locations(), self.session.map_player_pose(), career.as_ref())?;
                    }
                    self.accumulator = Duration::ZERO;
                    self.previous = now;
                    self.step = if pause.visible { FIXED_STEP } else {
                        self.session.sim_timer_hz().map_or(FIXED_STEP, timer_period)
                    };
                    return Ok(true);
                }
            }
            SelectedQueuedFrameOutcome::NoCommands {
                update_mask,
                delivered,
            } => {
                if !self.no_command_reported {
                    eprintln!(
                        "controlled input produced no simulation command: mask={update_mask} delivered={delivered}"
                    );
                    self.no_command_reported = true;
                }
            }
            SelectedQueuedFrameOutcome::Completed {
                update_mask,
                delivered,
                missing_receivers,
            } => {
                if missing_receivers != 0 {
                    return Err(format!(
                        "selected queued frame missed {missing_receivers} input receivers"
                    )
                    .into());
                }
                let visual_start = Instant::now();
                let pause = self.session.pause_menu_snapshot();
                let map = self.session.map_menu_snapshot();
                // Gameplay frames must not clone the frontend catalog or
                // acquire an extra PhysOut lock while the map is hidden.
                let locations = if map.visible { self.session.map_locations().to_vec() } else { Vec::new() };
                let player_pose = if map.visible { self.session.map_player_pose() } else { None };
                let career = self.session.take_career_hud();
                // DataSysMan::SubmitFrame: the completed tick's record for the restore.
                if let Some(restorer) = restorer.as_deref_mut() {
                    match self.session.presentation_submission()? {
                        Some(submission) => restorer.submit(std::sync::Arc::new(submission), due, Instant::now()),
                        None => restorer.reset(),
                    }
                }
                let mut visual = self.session.try_visual_snapshot()?;
                // Diagnostic controller driver only. Approach an actual published
                // pedestrian through ordinary pad input; never inject a contact,
                // teleport either entity, or change gameplay collision policy.
                if matches!(self.controls_demo, Some(ControlsDemo::PedestrianApproach)) && self.completed_frames >= 3000 {
                    let origin = visual.stream_focus_position;
                    let nearest = visual.pedestrians.iter().filter(|p|
                        (p.position[1] - origin[1]).abs() < 3.0)
                        .min_by(|a, b| {
                            let d = |p: [f32; 3]| (p[0]-origin[0]).powi(2)+(p[2]-origin[2]).powi(2);
                            d(a.position).total_cmp(&d(b.position))
                        });
                    if let Some(ped) = nearest {
                        let dx = ped.position[0] - origin[0];
                        let dz = ped.position[2] - origin[2];
                        let m = visual.provisional_clip_from_world;
                        let right_length = m[0].hypot(m[8]);
                        let forward_length = m[3].hypot(m[11]);
                        let x = (dx*m[0]+dz*m[8])/right_length;
                        let y = (dx*m[3]+dz*m[11])/forward_length;
                        let length = x.hypot(y);
                        if length.is_finite() && length > 0.001 {
                            self.pedestrian_approach_stick = [
                                (x/length*f32::from(i16::MAX)) as i16,
                                (y/length*f32::from(i16::MAX)) as i16,
                            ];
                        }
                        if self.completed_frames % 120 == 0 {
                            std::eprintln!("pedestrian approach frame={} player={origin:?} target={} position={:?} distance={} stick={:?}",
                                self.completed_frames, ped.entity_id, ped.position, dx.hypot(dz), self.pedestrian_approach_stick);
                        }
                    }
                }
                if skate2_core::diagnostics::lean_trace::enabled() {
                    skate2_core::diagnostics::lean_trace::record(
                        "position",
                        0,
                        self.completed_frames,
                        &visual.stream_focus_position,
                    );
                    if let Some((action, motion, physical)) =
                        self.session.diagnostic_state_ids()
                    {
                        skate2_core::diagnostics::lean_trace::record(
                            "states",
                            0,
                            self.completed_frames,
                            &[f32::from(action), f32::from(motion), physical as f32],
                        );
                    }
                }
                if std::env::var_os("SKATE2_TRACE_CITY_STREAM").is_some()
                    && self.completed_frames % 300 == 0
                {
                    eprintln!(
                        "city completed frame={} root={:?}",
                        self.completed_frames, visual.stream_focus_position
                    );
                }
                let snapshot_time = visual_start.elapsed();
                self.frame_timings.snapshot += snapshot_time;
                self.frame_timings.position = visual.stream_focus_position;
                let view_start = Instant::now();
                let vehicle_visuals = visual.vehicles.len();
                view.update(renderer, scene, now, &mut visual,
                    &pause, &map, &locations, player_pose, career.as_ref(), restorer.as_deref(), None)?;
                self.frame_timings.view += view_start.elapsed();
                if skate2_core::diagnostics::frame_profile::enabled() {
                    for (total, value) in self
                        .frame_timings
                        .core
                        .iter_mut()
                        .zip(skate2_core::diagnostics::frame_profile::take())
                    {
                        *total += value;
                    }
                }
                if skate2_core::diagnostics::frame_profile::enabled() {
                    let mut probe = crate::render_window::VehicleFrameProbe {
                        visuals: vehicle_visuals,
                        ..Default::default()
                    };
                    let mut nearest = f32::INFINITY;
                    for draw in &scene.dynamic_objects {
                        let Some(vehicle) = &draw.vehicle else {
                            continue;
                        };
                        probe.draws += 1;
                        let position = [
                            draw.part_transform[12],
                            draw.part_transform[13],
                            draw.part_transform[14],
                        ];
                        let distance: f32 = position
                            .iter()
                            .zip(visual.camera_world_position)
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum();
                        if distance < nearest {
                            nearest = distance;
                            probe.position = position;
                            probe.opacity = draw.opacity;
                            probe.palette_rows = vehicle.rows.len();
                            probe.palette_origin = std::array::from_fn(|axis| {
                                vehicle.rows.get(axis).map_or(f32::NAN, |row| row[3])
                            });
                            probe.palette_axis_lengths = std::array::from_fn(|axis| {
                                vehicle
                                    .rows
                                    .iter()
                                    .take(3)
                                    .map(|row| row[axis] * row[axis])
                                    .sum::<f32>()
                                    .sqrt()
                            });
                        }
                    }
                    self.frame_timings.vehicles = probe;
                }
                if std::env::var_os("SKATE2_PROFILE_REDRAW").is_some() {
                    eprintln!(
                        "diagnostic selected frame {}: tick={}ms snapshot={}ms view={}ms",
                        self.completed_frames + 1,
                        tick_time.as_secs_f64() * 1000.0,
                        snapshot_time.as_secs_f64() * 1000.0,
                        view_start.elapsed().as_secs_f64() * 1000.0,
                    );
                }
                let camera_update_counter = visual.camera_update_counter;
                self.session.acknowledge_frontend_scene_ready();
                if view.take_jacked_request() {
                    self.session.request_jacked_overlay()?;
                }
                // The sign-up poster screen's IntroComplete reaches its FE
                // state's OnIntroDone.
                if view.take_poster_intro_complete() {
                    self.session.acknowledge_career_poster_intro();
                }
                self.acknowledge_career_screens(view);
                self.completed_frames += 1;
                self.completed_counter
                    .store(self.completed_frames, Ordering::Relaxed);
                self.no_command_reported = false;
                if self.push_demo {
                    let (position, z_speed, deck_speed) = self
                        .session
                        .diagnostic_board_kinematics()
                        .ok_or("completed selected frame has no retained board kinematics")?;
                    if self.completed_frames == 33 {
                        self.push_start_position = Some(position);
                    }
                    if self.completed_frames == 34 {
                        let (action, _, physical) = self
                            .session
                            .diagnostic_state_ids()
                            .ok_or("push press completed without selected state IDs")?;
                        if action != 48 || physical != 100 {
                            return Err(format!(
                                "raw-A push press did not enter source-selected grounded Action48: Action={action} physical={physical}"
                            )
                            .into());
                        }
                    }
                    if self.completed_frames == 121 {
                        let start = self
                            .push_start_position
                            .ok_or("push replay has no pre-press board position")?;
                        let travel = start
                            .iter()
                            .zip(position)
                            .map(|(before, after)| (after - before).powi(2))
                            .sum::<f32>()
                            .sqrt();
                        if travel <= 0.3 || deck_speed <= 0.2 {
                            return Err(format!(
                                "raw-A push did not accelerate retained board: travel={travel} deck speed={deck_speed}"
                            )
                            .into());
                        }
                        eprintln!(
                            "live push corpus-equivalent movement: travel={travel} deck speed={deck_speed}"
                        );
                    }
                    eprintln!(
                        "live push completed frame {}: corpus step {:?}, PhysOut deck position={position:?} motion Z speed={z_speed} deck speed={deck_speed}",
                        self.completed_frames,
                        self.completed_frames.checked_sub(2),
                    );
                }
                if self.trace_selected_states {
                    eprintln!(
                        "selected offboard frame {}: {:?}",
                        self.completed_frames,
                        self.session.diagnostic_offboard_flags()
                    );
                    eprintln!(
                        "selected manual frame {}: {:?}",
                        self.completed_frames,
                        self.session.diagnostic_manual_flags()
                    );
                    eprintln!(
                        "selected board frame {}: {:?}",
                        self.completed_frames,
                        self.session.diagnostic_board_kinematics()
                    );
                    let (action, motion, physical) = self
                        .session
                        .diagnostic_state_ids()
                        .ok_or("completed selected frame has no state IDs")?;
                    eprintln!(
                        "selected states frame {}: Action={action} Motion={motion} physical={physical}",
                        self.completed_frames
                    );
                }
                if self.completed_frames == 1 || self.completed_frames.is_multiple_of(60) {
                    eprintln!(
                        "controlled frame {}: queued mask={update_mask} delivered={delivered} CameraMan update={}; city view and selected character share this producer projection",
                        self.completed_frames, camera_update_counter
                    );
                }
                // The completion limit is checked after EVERY completed tick,
                // before sampling another input or running frame N+1.
                if self.completion_reached() {
                    if let Some(start) = self.controlled_run_start.take() {
                        let wall_seconds = start.elapsed().as_secs_f64();
                        let simulation_seconds = self.completed_frames as f64 / 60.0;
                        // The first fixed tick is deliberately due immediately.
                        let scheduled_span_seconds =
                            self.completed_frames.saturating_sub(1) as f64 / 60.0;
                        eprintln!(
                            "controlled run complete: ticks={} simulation_seconds={simulation_seconds:.6} scheduled_span_seconds={scheduled_span_seconds:.6} elapsed_wall_seconds={wall_seconds:.6}; first tick immediate, completion includes tick/view work, excludes final present",
                            self.completed_frames,
                        );
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The pausing career screens' hand-offs: the unlock overlay's
    /// `IntroComplete` readies its FE state (`OnScreenReady`, then
    /// `OnIntroDone`) and its `Cash_SetCashAnimDone` lets the state take A;
    /// the Skate 101 screen's `IntroComplete` readies its state.
    fn acknowledge_career_screens(&mut self, view: &mut LiveSkatingView) {
        if view.take_unlock_intro_complete() {
            self.session.acknowledge_career_unlock_intro();
        }
        if view.take_cash_anim_done() {
            self.session.acknowledge_career_cash_anim_done();
        }
        if view.take_hom_results_intro_complete() {self.session.acknowledge_career_hom_results_intro();}
        if view.take_hom_annals_intro_complete() {self.session.acknowledge_career_hom_annals_intro();}
        if view.take_skate101_intro_complete() {
            self.session.acknowledge_career_skate101_intro();
        }
        if view.take_change_game_intro() {
            self.session.acknowledge_career_change_game_intro();
        }
        let (intro, outro) = view.take_end_options_screen();
        if intro {
            self.session.acknowledge_career_end_options_intro();
        }
        if outro {
            self.session.acknowledge_career_end_options_outro();
        }
        let (intro, outro) = view.take_sponsor_screen();
        if intro {
            self.session.acknowledge_career_sponsor_intro();
        }
        if outro {
            self.session.acknowledge_career_sponsor_outro();
        }
        if view.take_slappys_intro_complete() {
            self.session.acknowledge_career_slappys_intro();
        }
    }

    /// This redraw's frontend presentation and diagnostics after its ticks.
    /// Returns whether the presentation changed; `false` leaves the scene as the previous
    /// frame showed it.
    fn finish_redraw(
        &mut self,
        view: &mut LiveSkatingView,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        now: Instant,
        backlog_slack: u128,
    ) -> RenderResult<bool> {
        // PC: the update for the main menu's popup, and what the popup asked of the updater.
        if let Some(update) = self.update.as_mut() {
            let status = update.status();
            self.session.set_update_status(&status, update.current());
            if let Some(request) = self.session.take_update_request() {
                update.request(request);
            }
        }
        let replay_editor=self.session.replay_editor_snapshot();
        let replay_states=self.session.take_replay_editor_states();
        let reel=self.session.skate_reel_snapshot();
        view.set_replay_editor(replay_editor.as_ref(),&replay_states,reel.as_ref())?;
        view.set_trick_guide(self.session.trick_guide_snapshot().as_ref())?;
        let trick_guide_dt=if self.session.trick_guide_snapshot().is_some() {
            let dt=self.trick_guide_previous.map_or(0.,|last|now.saturating_duration_since(last).as_secs_f32());
            self.trick_guide_previous=Some(now);dt
        } else {self.trick_guide_previous=None;0.};
        {
            use skate2_core::ui::skate_reel::Action;
            let action=self.session.take_skate_reel_action();
            let result=(||->RenderResult<()> {
                match action {
                    Action::Browse(replay)|Action::Delete(replay,_)=>{
                        if let Action::Delete(_,slot)=action {crate::replay_media::delete(replay,slot)?;}
                        self.session.skate_reel_footage_loaded(replay,crate::replay_media::items(replay)?);
                    }
                    Action::Load(slot)=>self.session.queue_replay_file(crate::replay_media::read_replay(slot)?),
                    Action::Photo(slot)=>{crate::replay_media::photo_texture(slot)?;self.session.skate_reel_photo_opened(slot);}
                    Action::CreateASpot=>self.session.skate_reel_failed("Create-A-Spot features are not implemented yet."),
                    Action::None|Action::Back|Action::EditNickname=>(),
                }Ok(())
            })();
            if let Err(e)=result {self.session.skate_reel_failed(&e.to_string());}
        }
        if let Some(action)=self.session.take_replay_editor_save() {
            use skate2_core::ui::replay_editor::Action;
            match action {
                Action::Browse(replay)|Action::DeleteMedia(replay,_)=>{
                    let result=(||->RenderResult<_> {
                        if let Action::DeleteMedia(_,slot)=action {crate::replay_media::delete(replay,slot)?;}
                        crate::replay_media::items(replay)
                    })();
                    match result {
                        Ok(items)=>self.session.replay_editor_footage_loaded(replay,items),
                        Err(e)=>self.session.replay_editor_save_complete(replay,false,Err(e.to_string())),
                    }
                }
                Action::SaveReplay|Action::SavePhoto=>{
                    let replay=action==Action::SaveReplay;
                    let request=(||->Result<Option<crate::replay_media::Request>,String> {
                        let Some(slot)=crate::replay_media::next_slot(replay).map_err(|e|e.to_string())? else {return Ok(None)};
                        let bytes=if replay {Some(self.session.serialize_replay_editor()?.into())} else {None};
                        let editor=replay_editor.as_ref().ok_or("Replay Editor snapshot absent")?;
                        Ok(Some(crate::replay_media::Request {slot,replay:bytes,seconds:editor.encoded_seconds,
                            location:self.session.replay_editor_location()?,
                            video:if replay {editor.video} else {skate2_core::ui::replay_editor::VideoSettings::default()}}))
                    })();
                    match request {
                        Ok(Some(request))=>scene.replay_media=Some(request),
                        Ok(None)=>self.session.replay_editor_media_full(replay),
                        Err(message)=>self.session.replay_editor_save_complete(replay,false,Err(message)),
                    }
                }
                _=>return Err("unexpected Replay Editor host request".into()),
            }
        }
        let fade = scene.frontend_fade_alpha;
        let white = scene.nis_white_alpha;
        let mut changed = false;
        // FrontEndState_MoviePlayer's HUD state and FEMoviePlayer's subtitle line.
        if let Some(generation) = view.set_movie_state(self.session.movie_state_open(), self.session.movie_subtitle(),
            self.session.profile_settings().subtitles, self.session.movie_screen())? {
            self.session.acknowledge_movie_screen_loaded(generation);
        }
        view.set_hull_presentations(self.session.hull_presentations());
        if let Some(area) = view.presentation_stream_area() {
            self.session.set_presentation_stream_area(area);
        }
        if let Some(focuses)=view.replay_stream_focuses() {self.session.set_replay_stream_focuses(focuses);}
        // The map's travel and a front-end game start stream the destination under the fade.
        if let Some(position) = self.session.map_travel_position() {
            changed = true;
            let (disabled, xml) = self.session.map_travel_stream_policy()?;
            if view.prepare_map_travel(renderer, scene, position, disabled, xml)? {
                self.session.acknowledge_map_travel_presentation_ready();
            }
        }
        let guide_pause=self.session.pause_menu_snapshot();
        if let Some(frame)=self.session.main_menu_render_frame(self.accumulator.as_secs_f32().min(1.0/60.0))? {
            changed = true;
            let focus=self.session.main_menu_focus()?.ok_or("frontend focus absent")?;
            let states=self.session.take_freeskate_screen_states();
            let party_play_states=self.session.take_party_play_screen_states();
            let mut viewer=self.session.freeskate_viewer_visuals();
            viewer.extend(self.session.party_play_viewer_visuals());
            let ai=self.session.frontend_ai_skater_visuals()?;
            let (intro,freeskate_intro,party_play_intro)=
                view.update_main_menu(renderer,scene,now,focus,&frame,&states,&party_play_states,&viewer,&ai)?;
            self.session.acknowledge_main_menu(intro);
            let (extras, credits, done) = view.take_extras_events();
            self.session.acknowledge_extras(extras, credits, done);
            if freeskate_intro {
                self.session.acknowledge_freeskate_intro();
            }
            if party_play_intro {
                self.session.acknowledge_party_play_intro();
            }
            if view.take_online_match_intro() {
                self.session.online_match_screen_ready();
            }
            self.session.acknowledge_frontend_scene_ready();
            self.accumulator=Duration::from_nanos(
                u64::try_from(self.accumulator.as_nanos()%self.step.as_nanos()).expect("frontend fraction fits u64"));
        } else if let Some(editor_view)=self.session.edit_skater_view()? {
            changed = true;
            let (character,male,body,camera)=self.session.edit_skater_character()
                .ok_or("Edit Skater character missing")?;
            if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterRemoveShirt))
                && self.frontend_demo_ticks >= 1000 {
                if !character.recipe.components.iter().any(|c|
                    c.name == skate2_formats::cac_parts::SLOT_NAMES[23]) {
                    return Err("Edit Skater shirt diagnostic lost required torso".into());
                }
                if self.frontend_demo_ticks >= 1500 {
                    eprintln!("Edit Skater shirt diagnostic: removal completed with required torso rendered");
                }
            }
            if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterEyes)) && self.frontend_demo_ticks >= 1500 {
                if editor_view.menu != skate2_core::ui::edit_skater::EditSkaterMenu::Eyes || editor_view.selected != 0 {
                    return Err("eye-color diagnostic did not reach Eyes with only Color selected".into());
                }
                eprintln!("Eye-color diagnostic completed: {:?}", editor_view.rows.first());
            }
            // CAS camera constructor82f54320: normal near plane .05.
            view.update_editor(renderer,scene,now,character,male,body,camera,0.05)?;
            let (intro,outro)=view.update_editor_ui(renderer,scene,now,&editor_view)?;
            self.session.acknowledge_frontend_scene_ready();
            self.session.acknowledge_edit_skater_movie(intro,outro);
            if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterGender)) {
                if (1000..1200).contains(&self.frontend_demo_ticks) && male {
                    return Err("Edit Skater gender diagnostic did not render female".into());
                }
                if self.frontend_demo_ticks >= 1400 && !male {
                    return Err("Edit Skater gender diagnostic did not restore male outfit".into());
                }
                if self.frontend_demo_ticks >= 1500 {
                    eprintln!("Edit Skater gender diagnostic: female rendered, male outfit restored");
                }
            }
            if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterPopup))
                && self.frontend_demo_ticks >= 840 && editor_view.gender_confirmation.is_none() {
                return Err("Edit Skater popup diagnostic did not open confirmation".into());
            }
            if matches!(self.controls_demo, Some(ControlsDemo::EditSkater))
                && self.frontend_demo_ticks >= 600 {
                if !editor_view.visible || !intro {
                    return Err("Edit Skater diagnostic did not finish its intro".into());
                }
                eprintln!("Edit Skater diagnostic: room and character rendered; menu {:?}, {} rows, intro complete", editor_view.menu, editor_view.rows.len());
            }
            // Preserve the fractional frontend timer slot. Clearing it on
            // every fast redraw would prevent the next input tick entirely.
            self.accumulator=Duration::from_nanos(
                u64::try_from(self.accumulator.as_nanos()%self.step.as_nanos())
                    .expect("fractional frontend timer slot fits u64"));
        } else if let Some(guide)=self.session.trick_guide_render_frame(trick_guide_dt)? {
            changed=true;view.update_trick_guide(renderer,scene,now,&guide,&guide_pause)?;
            self.accumulator=Duration::from_nanos(u64::try_from(self.accumulator.as_nanos()%self.step.as_nanos()).expect("frontend fraction"));
        } else if self.session.replay_editor_open() {
            changed=true;
            let pause=self.session.pause_menu_snapshot();
            let map=self.session.map_menu_snapshot();
            let mut visual=self.session.try_replay_editor_visual_snapshot()?;
            view.update(renderer,scene,now,&mut visual,&pause,&map,&[],None,None,None,self.session.replay_restorer())?;
            if let Some(probe)=self.replay_probe.as_mut() {probe.rendered(replay_editor.as_ref().is_some_and(|s|s.ready));}
            // PostFX::Apply8284c848: ICELITE always selects the cinematic
            // pipeline, while 8284c914 excludes the Hall of Meat treatment.
            if let Some((_,postfx))=scene.postfx.as_mut() {postfx.cinematic=true;postfx.hom=None;postfx.tazer=false;}
            if scene.replay_media.is_some()||self.session.replay_export_active() {
                view.prepare_replay_capture(renderer,scene,replay_editor.as_ref().expect("editor open").watermark)?;
            } else {scene.capture_watermark=None;}
            self.accumulator=Duration::from_nanos(u64::try_from(self.accumulator.as_nanos()%self.step.as_nanos()).expect("frontend timer slot"));
        } else if !self.gameplay && (self.session.career_simulation_paused() || self.session.career_sign_up_choice_paused()) {
            changed = true;
            // kPAUSE_STATE_NIS, kPAUSE_STATE_CHALLENGE or the several-challenges popup's
            // kPAUSE_STATE_FRONTEND: no simulation update, but
            // GameModule::Update (0x8273edc8) still runs FrontEndManager::Update and
            // GameModule::Render (0x827320f0) still draws the retained world: through the ICE
            // camera once a scene exists, through the game camera and post-fx after Scene::Purge
            // (0x8286a908), as behind a film's end-of-challenge options, which the challenge pause
            // holds from its highlight reel to Continue or Retry.
            let career = self.session.take_career_hud();
            let pause = self.session.pause_menu_snapshot();
            let map = self.session.map_menu_snapshot();
            let mut visual = self.session.try_nis_visual_snapshot()?;
            let replay = self.session.replay_restorer();
            view.update(renderer, scene, now, &mut visual, &pause, &map, &[], None, career.as_ref(), None, replay)?;
            // A replay return may retain the challenge's pause as well as
            // StartHub's. This branch has prepared the restored world too.
            self.session.acknowledge_frontend_scene_ready();
            self.acknowledge_career_screens(view);
            self.accumulator = Duration::from_nanos(
                u64::try_from(self.accumulator.as_nanos() % self.step.as_nanos())
                    .expect("fractional timer slot fits u64 nanoseconds"));
        } else if !self.gameplay && self.session.career_overlay_open() {
            changed = true;
            // FrontEndState_Unlock pauses the game; only the front end runs.
            if self.session.frontend_scene_reveal_pending() {
                // The Replay Editor pops back to the Hall of Meat results: the retained
                // world is drawn again through the game camera before the reveal.
                let career = self.session.take_career_hud();
                let pause = self.session.pause_menu_snapshot();
                let map = self.session.map_menu_snapshot();
                let mut visual = self.session.try_nis_visual_snapshot()?;
                let replay = self.session.replay_restorer();
                view.update(renderer, scene, now, &mut visual, &pause, &map, &[], None, career.as_ref(), None, replay)?;
                self.session.acknowledge_frontend_scene_ready();
            } else if let Some(career) = self.session.take_career_hud() {
                view.update_career_frontend(renderer, scene, now, &career)?;
            }
            self.acknowledge_career_screens(view);
            self.accumulator = Duration::from_nanos(
                u64::try_from(self.accumulator.as_nanos() % self.step.as_nanos())
                    .expect("fractional timer slot fits u64 nanoseconds"));
        } else if self.session.pause_menu_snapshot().visible || (!self.gameplay && self.session.online_hub_open()) {
            changed = true;
            // The StartHub, and over a single player game the online hub and its screens
            // (gFEStateInfo rows 44, 68 and 69, mbPause), pause the game: the front end keeps
            // updating and drawing over the retained world.
            let career = self.session.take_career_hud();
            if self.session.frontend_scene_reveal_pending() {
                // Replay2 OnUpdate826799f8 stops playback and pops StartHub
                // after the live focus reloads. Its retained world must be
                // drawn with the game camera/PostFX before revealing the hub;
                // a paused return cannot wait for another simulation tick.
                let pause=self.session.pause_menu_snapshot();
                let map=self.session.map_menu_snapshot();
                let mut visual=self.session.try_nis_visual_snapshot()?;
                view.update(renderer,scene,now,&mut visual,&pause,&map,self.session.map_locations(),
                    self.session.map_player_pose(),career.as_ref(),None,self.session.replay_restorer())?;
                self.session.acknowledge_frontend_scene_ready();
            } else {
                view.update_pause(renderer, scene, now, &self.session.pause_menu_snapshot(),
                    &self.session.map_menu_snapshot(), self.session.map_locations(), self.session.map_player_pose(), career.as_ref())?;
            }
            // Input may be bounded after a long UI stall, but no simulation
            // catch-up debt belongs to a paused period.
            self.accumulator = Duration::from_nanos(
                u64::try_from(self.accumulator.as_nanos() % self.step.as_nanos())
                    .expect("fractional timer slot fits u64 nanoseconds"));
        } else if self.session.movie_state_open() {
            changed = true;
            // FE state 28 pauses the game; the front end keeps drawing, over a movie it plays
            // under the front end.
            view.update_movie_frontend(renderer, scene, now)?;
        }
        // The StartHub's, Options' and the music player's screens take input from their
        // IntroComplete; the music player's OnLoaded runs as its screen links.
        let screens = view.take_screen_events();
        self.session.skate_reel_screen_ready(screens.skate_reel_intro,screens.replay_footage_intro);
        if screens.replay_editor_intro {self.session.replay_editor_intro_complete()?;}
        if screens.trick_guide_intro {self.session.trick_guide_intro_complete();}
        if screens.contacts_intro {self.session.contacts_intro_complete();}
        if screens.skate_profile_intro {self.session.skate_profile_intro_complete();}
        if screens.replay_editor_ready {self.session.replay_editor_animation_ready();}
        if screens.replay_footage_intro {self.session.replay_editor_footage_ready();}
        self.session.acknowledge_screens(screens.start_hub_intro, screens.options_intro,
            screens.music_player_loaded, screens.music_player_intro);
        // SKATE2_DIAGNOSTIC_SUBTITLE_SHOT=<n>[,<ms>]: the career demo completes, and captures, the
        // given milliseconds (500 by default) after the n-th front-end subtitle line comes up,
        // shown or not.
        if let Some(n) = std::env::var("SKATE2_DIAGNOSTIC_SUBTITLE_SHOT").ok().and_then(|n| n.split(',').next()?.parse::<u32>().ok()) {
            let (nis, shown) = view.subtitle_lines();
            let line = if self.session.movie_state_open() { self.session.movie_subtitle().text } else { nis };
            subtitle_shot(n, line, shown, self.session.profile_settings().subtitles);
        }
        if matches!(self.controls_demo,Some(ControlsDemo::MainMenu|ControlsDemo::MainMenuRoundtrip)) && self.completion_reached() {
            let frame=self.session.main_menu_frame()?.ok_or("main menu diagnostic ended outside frontend")?;
            if !frame.menu_visible {return Err("main menu diagnostic did not finish camera animation".into());}
            if frame.selected!=0 {return Err("main menu diagnostic did not restore Continue Career selection".into());}
            if matches!(self.controls_demo,Some(ControlsDemo::MainMenuRoundtrip)) && self.completed_frames==0 {
                return Err("main menu roundtrip never resumed gameplay".into());
            }
            eprintln!("Main menu diagnostic completed: original camera settled, selected={}, gameplay ticks={}",frame.selected,self.completed_frames);
        }
        if matches!(self.controls_demo,Some(ControlsDemo::DisplaySettings)) && self.completion_reached() {
            let display=self.session.options_snapshot();
            let wall = std::env::var_os("SKATE2_DIAGNOSTIC_DISPLAY_WALL").is_some();
            if (!wall && (!self.session.pause_menu_snapshot().visible || self.completed_frames==0))
                || (wall && self.completed_frames!=0)
                || display.page!=skate2_core::ui::options::Page::Display
                || display.values.postfx || display.values.fisheye
                || display.values.frame_rate_cap!=skate2_core::ui::options::FrameRateCap::Fps60 {
                return Err(format!(
                    "Display Settings entry / independent toggles failed: paused {} frames {} page {:?} values {:?}",
                    self.session.pause_menu_snapshot().visible, self.completed_frames, display.page, display.values
                ).into());
            }
            eprintln!("Display Settings: {} entry, both independent toggles and the 60 cap verified; gameplay ticks={}",
                if wall {"wall"} else {"pause"}, self.completed_frames);
        }
        renderer.set_frontend_fade(scene, self.session.frontend_fade_alpha())?;
        renderer.set_nis_white(scene, self.session.nis_bloom_to_white())?;
        // AptRenderingIntegration::Render2D's AptMovieIntegration::Render, each render.
        let requests = self.session.take_movie_requests();
        let movie = view.update_movie(renderer, scene, requests, self.session.movie_foreground(), self.session.movie_background_forced());
        self.session.set_movie_presentation(movie);
        changed |= scene.movie.is_some();
        scene.stereoscopy = self.session.stereoscopy_enabled() && self.session.main_menu_frame()?.is_none();
        // Presentation happens once when this callback returns; objects contain
        // the latest completed tick. Keep uploads at each completion because
        // a later NoCommands outcome makes the facade snapshot unavailable.
        if matches!(self.controls_demo, Some(ControlsDemo::EditSkater))
            && self.frontend_demo_ticks >= 600 && self.session.edit_skater_view()?.is_none() {
            return Err("Edit Skater diagnostic never entered the editor".into());
        }
        if matches!(self.controls_demo, Some(ControlsDemo::EditSkaterRoundtrip))
            && self.frontend_demo_ticks >= 1500 {
            if self.session.edit_skater_view()?.is_some() || self.session.pause_menu_snapshot().visible {
                return Err("Edit Skater roundtrip did not return to gameplay".into());
            }
            eprintln!("Edit Skater roundtrip: returned to live gameplay after Finish/save");
        }
        if matches!(self.controls_demo, Some(ControlsDemo::MapTeleport)) && self.frontend_demo_ticks >= 1500 {
            if self.session.pause_menu_snapshot().visible || self.session.frontend_fade_alpha() != 0. {
                return Err("Map teleport diagnostic did not return to revealed gameplay".into());
            }
            eprintln!("Map teleport diagnostic: returned to revealed gameplay");
        }
        let remaining = self.accumulator.as_nanos() / self.step.as_nanos();
        if remaining > backlog_slack && !self.completion_reached() {
            self.backlogged_redraws = self.backlogged_redraws.saturating_add(1);
            if self.backlogged_redraws == 1 || self.backlogged_redraws.is_multiple_of(60) {
                eprintln!(
                    "controlled host backlog: {remaining} fixed ticks retained after budget {MAX_TICKS_PER_REDRAW}; no simulation time discarded"
                );
            }
        } else {
            self.backlogged_redraws = 0;
        }
        Ok(changed || scene.frontend_fade_alpha != fade || scene.nis_white_alpha.to_bits() != white.to_bits())
    }
}

/// How long an idle window waits for a tick before checking its window events.
const IDLE_WAIT: Duration = Duration::from_millis(50);

/// One attempted tick, produced on whichever thread ran it.
struct TickRecord {
    /// When the fixed step made this tick due.
    due: Instant,
    started: Instant,
    was_paused: bool,
    outcome: Result<SelectedQueuedFrameOutcome, String>,
    tick_time: Duration,
    core: [Duration; skate2_core::diagnostics::frame_profile::COUNT],
}

impl TickRecord {
    fn failed(error: String) -> Self {
        let now = Instant::now();
        Self {
            due: now,
            started: now,
            was_paused: false,
            outcome: Err(error),
            tick_time: Duration::ZERO,
            core: [Duration::ZERO; skate2_core::diagnostics::frame_profile::COUNT],
        }
    }
}

/// `SKATE2_TICK_TRACE`: each consumed tick's due, start, finish and consumption times in
/// milliseconds since the host's start, written as CSV when the client closes.
/// `SKATE2_PRESENT_TRACE`: each SIM-thread tick's due and consumption times and the end of
/// the present that first showed it.
struct TickTrace {
    path: std::path::PathBuf,
    header: &'static str,
    rows: Vec<[f64; 4]>,
}

impl TickTrace {
    fn from_environment() -> Option<Self> {
        Some(Self {
            path: std::env::var_os("SKATE2_TICK_TRACE")?.into(),
            header: "due_ms,start_ms,finish_ms,consumed_ms",
            rows: Vec::with_capacity(1 << 16),
        })
    }

    fn presents_from_environment() -> Option<Self> {
        Some(Self {
            path: std::env::var_os("SKATE2_PRESENT_TRACE")?.into(),
            header: "due_ms,consumed_ms,presented_ms,unused",
            rows: Vec::with_capacity(1 << 16),
        })
    }

    /// `SKATE2_RESTORE_TRACE`: each restored frame's start, the moment it shows (in ticks)
    /// and its camera's x and z.
    fn restores_from_environment() -> Option<Self> {
        Some(Self {
            path: std::env::var_os("SKATE2_RESTORE_TRACE")?.into(),
            header: "frame_ms,shown_tick,camera_x,camera_z",
            rows: Vec::with_capacity(1 << 16),
        })
    }

    fn push_restore(&mut self, started: Instant, restorer: &PresentationRestorer) {
        let (Some(shown), Some(camera)) = (restorer.shown_moment(), restorer.camera()) else {
            return;
        };
        if self.rows.len() < 1 << 20 {
            let start = skate2_core::diagnostics::start();
            self.rows.push([
                started.saturating_duration_since(start).as_secs_f64() * 1000.,
                shown,
                f64::from(camera.position[0]),
                f64::from(camera.position[2]),
            ]);
        }
    }

    fn push(&mut self, times: [Instant; 4]) {
        if self.rows.len() < 1 << 20 {
            let start = skate2_core::diagnostics::start();
            self.rows
                .push(times.map(|time| time.saturating_duration_since(start).as_secs_f64() * 1000.));
        }
    }
}

impl Drop for TickTrace {
    fn drop(&mut self) {
        use std::io::Write as _;
        let write = || -> std::io::Result<()> {
            let mut file = std::io::BufWriter::new(std::fs::File::create(&self.path)?);
            writeln!(file, "{}", self.header)?;
            for [due, start, finish, consumed] in &self.rows {
                writeln!(file, "{due:.3},{start:.3},{finish:.3},{consumed:.3}")?;
            }
            file.flush()
        };
        if let Err(error) = write() {
            std::eprintln!("Could not save tick trace {}: {error}", self.path.display());
        }
    }
}

/// The original runs SIM and PRES on separate threads. Gameplay ticks run on a
/// SIM thread while this thread presents; each completed tick is consumed here
/// (visual snapshot, view upload, acknowledgements) before the next one may
/// start, so tick order and per-tick input are those of the serial loop.
/// `SKATE2_SERIAL_SIM` or a lean trace keeps every tick on this thread.
struct LiveSkatingWindow {
    view: LiveSkatingView,
    shared: Arc<SimShared>,
    worker: Option<std::thread::JoinHandle<()>>,
    frame_timings: ClientFrameTimings,
    /// Latest presented fade alpha not yet delivered (a tick held the session).
    presented_alpha: Option<f32>,
    completion: bool,
    completion_bound: bool,
    /// The session's Exit to Desktop is ready (`exit_to_desktop_ready`).
    exit: bool,
    present_trace: Option<TickTrace>,
    /// The consumed tick's due and consumption times until its frame presents.
    unpresented: Option<(Instant, Instant)>,
    /// The last frame had nothing new to show, so the next waits for a tick.
    idle: bool,
    /// `DataSysMan`'s presentation side: the submitted ticks, the clock and the restored
    /// camera and skaters each frame draws.
    restorer: PresentationRestorer,
    /// Frames between ticks draw from the restore without the SIM thread's lock.
    restoring: bool,
    /// `SKATE2_NO_PRESENTATION_RESTORE`: draw each tick as it completes instead, for
    /// captures that do not depend on when the frame drew.
    restore_disabled: bool,
    /// When the previous frame started: `AppMan::RenderLoop`'s stopwatch.
    last_render: Option<Instant>,
    last_presentation: Duration,
    last_finish: Duration,
    last_presented: Option<Instant>,
    presentation_started: Option<Instant>,
    restore_trace: Option<TickTrace>,
}

struct SimShared {
    slot: Mutex<SimSlot>,
    wake: Condvar,
    /// Signalled when the SIM thread leaves a tick for the window.
    ready: Condvar,
    /// A tick waits for the window: set with `ready`, cleared when it is consumed.
    waiting: AtomicBool,
}

struct SimSlot {
    client: LiveSkatingClient,
    /// Presentation navigation after the last consumed tick.
    navigation: Vec<skate2_core::presentation::PresentationNavigationArena>,
    /// A SIM-thread tick awaiting consumption.
    record: Option<TickRecord>,
    stop: bool,
}

impl LiveSkatingWindow {
    fn new(client: LiveSkatingClient, view: LiveSkatingView) -> RenderResult<Self> {
        let completion_bound = client.completion_bound_active();
        let shared = Arc::new(SimShared {
            slot: Mutex::new(SimSlot {
                client,
                navigation: Vec::new(),
                record: None,
                stop: false,
            }),
            wake: Condvar::new(),
            ready: Condvar::new(),
            waiting: AtomicBool::new(false),
        });
        let worker = if std::env::var_os("SKATE2_SERIAL_SIM").is_none()
            && !skate2_core::diagnostics::lean_trace::enabled()
        {
            let shared = Arc::clone(&shared);
            Some(
                std::thread::Builder::new()
                    .name("sk8 SIM".to_owned())
                    .stack_size(16 << 20)
                    .spawn(move || run_sim_thread(&shared))?,
            )
        } else {
            None
        };
        Ok(Self {
            view,
            shared,
            worker,
            frame_timings: ClientFrameTimings::default(),
            presented_alpha: None,
            completion: false,
            completion_bound,
            exit: false,
            present_trace: TickTrace::presents_from_environment(),
            unpresented: None,
            idle: false,
            restorer: PresentationRestorer::new(),
            restoring: false,
            restore_disabled: std::env::var_os("SKATE2_NO_PRESENTATION_RESTORE").is_some(),
            last_render: None,
            last_presentation: Duration::ZERO,
            last_finish: Duration::ZERO,
            last_presented: None,
            presentation_started: None,
            restore_trace: TickTrace::restores_from_environment(),
        })
    }
}

/// Runs each due gameplay tick, then waits until it has been consumed.
fn run_sim_thread(shared: &SimShared) {
    let Ok(mut slot) = shared.slot.lock() else {
        return;
    };
    loop {
        if slot.stop {
            return;
        }
        if slot.record.is_none() && slot.client.threaded_tick_allowed() {
            let SimSlot { client, navigation, record, .. } = &mut *slot;
            client.advance_clock(Instant::now());
            if client.accumulator >= client.step {
                client.accumulator -= client.step;
                *record = Some(match client.stage_tick(navigation) {
                    Ok(was_paused) => client.run_tick(was_paused),
                    Err(error) => TickRecord::failed(error.to_string()),
                });
                shared.waiting.store(true, Ordering::Release);
                shared.ready.notify_one();
                continue;
            }
            let wait = client.step - client.accumulator;
            drop(slot);
            std::thread::sleep(wait);
            let Ok(next) = shared.slot.lock() else {
                return;
            };
            slot = next;
            continue;
        }
        let Ok((next, _)) = shared.wake.wait_timeout(slot, Duration::from_millis(100)) else {
            return;
        };
        slot = next;
    }
}

impl RenderWindowClient for LiveSkatingWindow {
    fn toggle_fullscreen(&mut self) {
        let toggled = match self.shared.slot.lock() {
            Ok(mut slot) => slot.client.session.toggle_fullscreen().map_err(|error| error.to_string()),
            Err(_) => Err("the simulation stopped".to_owned()),
        };
        if let Err(error) = toggled {
            eprintln!("fullscreen: {error}");
        }
    }
    fn replay_media_saved(&mut self,replay:bool,result:Result<String,String>) {
        // FEReplay2's bShowReplayInfo is process-wide, not a count of files.
        static SHOWN:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
        let first=replay&&result.is_ok()&&!SHOWN.swap(true,Ordering::Relaxed);
        if let Ok(mut shared)=self.shared.slot.lock() {
            shared.client.session.replay_editor_save_complete(replay,first,result);
        }
    }
    fn refresh_media_capture(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene)->RenderResult<()> {
        let mut slot=self.shared.slot.lock().map_err(|_|crate::panic_report::sim_error())?;
        self.view.begin_ui_geometry_batch();
        slot.client.finish_redraw(&mut self.view,renderer,scene,Instant::now(),1)?;
        if let Some(editor)=slot.client.session.replay_editor_snapshot() {
            self.view.prepare_replay_capture(renderer,scene,editor.watermark)?;
        }
        self.view.finish_ui_geometry_batch(renderer,scene)?;
        Ok(())
    }
    fn media_capture_ready(&self)->bool {self.view.media_scene_ready()}
    fn begin_replay_export(&mut self)->RenderResult<()> {
        self.shared.slot.lock().map_err(|_|crate::panic_report::sim_error())?.client.session.begin_replay_export()?;
        self.view.begin_export_audio();Ok(())
    }
    fn prepare_replay_export_frame(&mut self,renderer:&GpuRenderer,scene:&mut GpuScene,frame:u64)->RenderResult<skate2_core::ui::replay_editor::ExportStatus> {
        let status=self.shared.slot.lock().map_err(|_|crate::panic_report::sim_error())?.client.session.prepare_replay_export_frame(frame)?;
        self.refresh_media_capture(renderer,scene)?;
        use skate2_core::ui::replay_editor::ExportStatus;
        Ok(if status==ExportStatus::Frame&&!self.view.media_scene_ready() {ExportStatus::Waiting} else {status})
    }
    fn replay_export_audio(&mut self)->RenderResult<Vec<f32>> {
        let (state,world)=self.shared.slot.lock().map_err(|_|crate::panic_report::sim_error())?.client.audio_inputs();
        Ok(self.view.export_audio_frame(state,world))
    }
    fn end_replay_export(&mut self)->RenderResult<()> {
        let result=self.shared.slot.lock().map_err(|_|crate::panic_report::sim_error())?.client.session.end_replay_export();
        self.view.end_export_audio();result?;Ok(())
    }
    fn replay_media_cancelled(&mut self) {
        if let Ok(mut slot)=self.shared.slot.lock(){slot.client.session.replay_editor_save_cancelled();}
    }
    fn career_photo_saved(&mut self, slot: u32) {
        if let Ok(mut shared) = self.shared.slot.lock() {
            shared.client.session.acknowledge_career_photo_save(slot);
        }
    }

    fn frame_timings(&self) -> ClientFrameTimings {
        self.frame_timings
    }

    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<GpuScene> {
        let mut slot = self.shared.slot.lock().map_err(|_| crate::panic_report::sim_error())?;
        slot.client.initialize(&mut self.view, renderer)
    }

    fn text_entry_active(&self) -> bool {
        self.shared.slot.lock().is_ok_and(|slot| slot.client.session.extras_text_entry_active())
    }

    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()> {
        if !matches!(event, WindowEvent::Focused(_) | WindowEvent::KeyboardInput { .. }) {
            return Ok(());
        }
        if matches!(event, WindowEvent::Focused(_)) {
            self.last_presentation = Duration::ZERO;
            self.last_finish = Duration::ZERO;
            self.last_presented = None;
            self.presentation_started = None;
        }
        let mut slot = self.shared.slot.lock().map_err(|_| crate::panic_report::sim_error())?;
        slot.client.window_event(event)
    }

    fn completion_reached(&self) -> bool {
        self.completion
    }

    fn exit_requested(&self) -> bool {
        self.exit
    }

    fn completion_bound_active(&self) -> bool {
        self.completion_bound
    }

    fn record_presented_completion(&mut self) {
        if let Ok(mut slot) = self.shared.slot.lock() {
            slot.client.record_presented_completion();
        }
    }

    fn frame_presented(&mut self, scene: &GpuScene) {
        let presented = Instant::now();
        if let Some(started) = self.presentation_started.take() {
            self.last_presentation = presented.saturating_duration_since(started);
            self.frame_timings.post_tick_presentation = self.last_presentation;
        }
        self.last_presented = Some(presented);
        if let (Some(trace), Some((due, consumed))) = (&mut self.present_trace, self.unpresented.take()) {
            let now = Instant::now();
            trace.push([due, consumed, now, now]);
        }
        let alpha = scene.frontend_fade_alpha;
        match self.shared.slot.try_lock() {
            Ok(mut slot) => {
                if let Some(earlier) = self.presented_alpha.take() {
                    slot.client.frame_presented(earlier);
                }
                slot.client.frame_presented(alpha);
            }
            Err(_) => self.presented_alpha = Some(alpha),
        }
    }

    fn frame(
        &mut self,
        renderer: &GpuRenderer,
        scene: &mut GpuScene,
        _now: Instant,
    ) -> RenderResult<()> {
        scene.tick_due = None;
        scene.unchanged = false;
        // AppMan::RenderLoop 0x82989fc0: the time since the previous frame, at most
        // kMaxDtBeforeRenderDrop, steps the presentation's clock.
        let started = Instant::now();
        let dt = self.last_render.map_or(0.0, |last| {
            started
                .saturating_duration_since(last)
                .as_secs_f32()
                .min(skate2_core::presentation_restore::clock::MAX_DT_BEFORE_RENDER_DROP)
        });
        self.last_render = Some(started);
        // Include event-loop/frame-cap waiting: a 30 Hz presentation must not
        // accidentally limit the 60 Hz simulation to one update per image.
        let presentation_cost = self.last_presentation.saturating_add(
            self.last_presented.map_or(Duration::ZERO, |last| started.saturating_duration_since(last)));
        self.presentation_started = Some(started);
        // Between ticks the world is the restore's: no tick waits, so the SIM thread
        // keeps its lock while this frame draws.
        if self.restoring && !self.idle && !self.shared.waiting.load(Ordering::Acquire) {
            self.frame_timings = ClientFrameTimings {
                position: self.frame_timings.position,
                ..Default::default()
            };
            if self.restorer.step(dt, started)? {
                self.view.update_frame(renderer, scene, started, &self.restorer)?;
                if let Some(trace) = &mut self.restore_trace {
                    trace.push_restore(started, &self.restorer);
                }
            } else {
                scene.unchanged = true;
                self.idle = true;
            }
            return Ok(());
        }
        self.view.begin_ui_geometry_batch();
        let lock_started = Instant::now();
        let mut slot = self.shared.slot.lock().map_err(|_| crate::panic_report::sim_error())?;
        let lock_wait = lock_started.elapsed();
        let ready_started = Instant::now();
        // Wait only when a SIM worker can actually produce the missing tick.
        // Serial/frontend paths must run their own update below, not wait 50ms
        // for a worker which is disabled or sleeping on the gameplay gate.
        if self.idle && self.worker.is_some() && slot.client.threaded_tick_allowed() {
            slot = self
                .shared
                .ready
                .wait_timeout_while(slot, IDLE_WAIT, |slot| slot.record.is_none() && !slot.stop)
                .map_err(|_| crate::panic_report::sim_error())?
                .0;
        }
        let ready_wait = ready_started.elapsed();
        // Read under the lock: the SIM thread may have advanced the clock.
        let now = Instant::now();
        slot.client.frame_timings = ClientFrameTimings {
            sim_lock_wait: lock_wait,
            sim_ready_wait: ready_wait,
            position: slot.client.frame_timings.position,
            vehicles: slot.client.frame_timings.vehicles,
            ..Default::default()
        };
        if let Some(alpha) = self.presented_alpha.take() {
            slot.client.frame_presented(alpha);
        }
        let threaded = self.worker.is_some();
        if self.restore_disabled {
            self.restorer.reset();
        }
        let mut handoff_completed = None;
        let consumed = if slot.record.is_some() {
            let mut ticks = 0;
            let mut catch_up_budget = 1;
            loop {
                let record = slot.record.take().ok_or("SIM handoff lost its tick")?;
                self.shared.waiting.store(false, Ordering::Release);
                if ticks == 0 {
                    scene.tick_due = Some(record.due);
                    self.unpresented = Some((record.due, Instant::now()));
                }
                ticks += 1;
                slot.client.frame_timings.worker_ticks += 1;
                let tick_cost = record.tick_time;
                let completion_started = Instant::now();
                let stop = slot.client.complete_tick(
                    &mut self.view, renderer, scene, now, record, Some(&mut self.restorer),
                )?;
                let completion_cost = completion_started.elapsed();
                slot.client.frame_timings.tick_completion += completion_cost;
                handoff_completed = Some(Instant::now());
                if ticks == 1 {
                    catch_up_budget = pipeline_tick_budget(slot.client.step, tick_cost,
                        completion_cost, self.last_finish, presentation_cost);
                    slot.client.frame_timings.catch_up_budget = catch_up_budget;
                }
                slot.client.advance_clock(Instant::now());
                if stop || ticks >= catch_up_budget
                    || !catch_up_due(slot.client.accumulator, slot.client.step, ticks)
                    || !slot.client.threaded_tick_allowed()
                {
                    break;
                }
                // AppMan::MainLoop 0x82989ae0 owns Update separately from
                // RenderLoop 0x82989fc0. Keep overdue physics on SIM as well.
                // Preserve each completion/acknowledgement and navigation
                // update before permitting another tick. The condvar releases
                // the session mutex; no physics runs concurrently with it.
                // Retain bounded catch-up: one tick per draw would slow the
                // simulation whenever presentation falls below the timer rate.
                slot.navigation = self.view.navigation();
                self.shared.wake.notify_one();
                let wait_started = Instant::now();
                while slot.record.is_none() && !slot.stop {
                    // A timeout also rechecks mutex poisoning if SIM panics
                    // before it can notify; otherwise retain the pending tick.
                    slot = self.shared.ready.wait_timeout_while(
                        slot, Duration::from_millis(100),
                        |slot| slot.record.is_none() && !slot.stop,
                    ).map_err(|_| crate::panic_report::sim_error())?.0;
                }
                slot.client.frame_timings.sim_ready_wait += wait_started.elapsed();
                if slot.stop {
                    return Err("SIM thread stopped during catch-up".into());
                }
            }
            slot.client.finish_redraw(&mut self.view, renderer, scene, now, 1)?;
            true
        } else if threaded && slot.client.threaded_tick_allowed() {
            scene.unchanged = !slot.client.finish_redraw(&mut self.view, renderer, scene, now, 1)?;
            false
        } else {
            slot.client.frame(&mut self.view, renderer, scene, now)?;
            true
        };
        let SimSlot { client, navigation, .. } = &mut *slot;
        let wake = threaded && client.threaded_tick_allowed();
        if std::env::var_os("SKATE2_TRACE_WAKE").is_some() {
            static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
            let state = format!(
                "wake={wake} ready={} gameplay={} pause={} unchanged={} restoring={}",
                client.ready,
                client.gameplay,
                client.session.pause_menu_snapshot().visible,
                scene.unchanged,
                self.restoring
            );
            if let Ok(mut last) = LAST.lock()
                && *last != state
            {
                eprintln!("wake trace {:.3}: {state}", skate2_core::diagnostics::start().elapsed().as_secs_f64());
                *last = state;
            }
        }
        if wake && consumed {
            *navigation = self.view.navigation();
        }
        self.completion = client.completion_reached();
        self.exit = client.session.exit_to_desktop_ready();
        // Include wall time since the most recent clock advance. This is a
        // diagnostic only: observing debt never consumes or discards a tick.
        client.frame_timings.simulation_debt = if wake {
            client.accumulator.saturating_add(Instant::now().saturating_duration_since(client.previous))
        } else {
            Duration::ZERO
        };
        self.frame_timings = client.frame_timings;
        self.idle = scene.unchanged;
        // A tick shown through the restore draws the restored moment of this frame; the
        // pacing of whole ticks does not apply.
        if !wake {
            self.restorer.reset();
        }
        let pending_draw = self.view.take_pending_draw();
        if pending_draw {
            scene.tick_due = None;
            self.restorer.step(dt, Instant::now())?;
            scene.unchanged = false;
            self.idle = false;
        }
        self.restoring = wake && self.restorer.is_running() && self.view.draws_between_ticks();
        client.session.set_presentation_cloth(self.restoring);
        drop(slot);
        if wake {
            self.shared.wake.notify_one();
        }
        if let Some(completed) = handoff_completed {
            self.last_finish = completed.elapsed();
            self.frame_timings.tick_finish = self.last_finish;
        }
        self.presentation_started = Some(Instant::now());
        // This only uses the window-owned view, restore and GPU scene. Wake
        // SIM before palette/cloth preparation and uploads so its next tick
        // overlaps this work as well as command recording/presentation.
        if pending_draw {
            self.view.update_frame(renderer, scene, now, &self.restorer)?;
            if let Some(trace) = &mut self.restore_trace {
                trace.push_restore(started, &self.restorer);
            }
        }
        self.view.finish_ui_geometry_batch(renderer, scene)?;
        Ok(())
    }
}

impl Drop for LiveSkatingWindow {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.shared.slot.lock() {
            slot.stop = true;
        }
        self.shared.wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod fixed_step_tests {
    use super::*;

    fn ticks_due(mut debt: Duration, step: Duration) -> (u32, Duration) {
        let mut ticks = 0;
        while tick_due(debt, step, ticks) {
            ticks += 1;
            debt -= step;
        }
        (ticks, debt)
    }

    #[test]
    fn bounded_redraws_retain_whole_debt_and_fractional_remainder() {
        let fraction = Duration::from_nanos(123);
        let mut debt = FIXED_STEP * 11 + fraction;
        let mut executed = 0;
        for expected in [4, 4, 3] {
            let (due, rest) = ticks_due(debt, FIXED_STEP);
            assert_eq!(due, expected);
            executed += due;
            debt = rest;
        }
        assert_eq!(executed, 11);
        assert_eq!(debt, fraction);
        assert_eq!(ticks_due(debt, FIXED_STEP).0, 0);
    }

    #[test]
    fn scheduler_does_not_wait_for_four_ticks_and_caps_large_stalls() {
        assert_eq!(
            ticks_due(FIXED_STEP - Duration::from_nanos(1), FIXED_STEP).0,
            0
        );
        assert_eq!(ticks_due(FIXED_STEP, FIXED_STEP).0, 1);
        assert_eq!(ticks_due(FIXED_STEP * 2, FIXED_STEP).0, 2);
        assert_eq!(ticks_due(Duration::MAX, FIXED_STEP).0, MAX_TICKS_PER_REDRAW);
    }

    #[test]
    fn slow_motion_timer_uses_native_hundred_nanosecond_periods() {
        assert_eq!(timer_period(60), FIXED_STEP);
        assert_eq!(timer_period(30), Duration::from_nanos(33_333_300));
        assert_eq!(timer_period(45), Duration::from_nanos(22_222_200));
    }
}
