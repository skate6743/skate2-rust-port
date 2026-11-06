//! Optional host timings, consumed by the window's in-memory frame capture.
//! No file/console output and no changes to native timing or scheduling.
use std::{
    cell::RefCell,
    sync::OnceLock,
    time::{Duration, Instant},
};

pub const COUNT: usize = 51;
pub const NAMES: [&str; COUNT] = [
    "queued",
    "actor_total",
    "pre_solve",
    "solver",
    "post_solve",
    "camera_manager_total",
    "prediction",
    "breadcrumb",
    "presentation",
    "recipes",
    "living_world1",
    "living_world2",
    "living_world_post",
    "vehicle_ground",
    "solver_prepare",
    "dmo_ground",
    "dmo_pairs",
    "dmo_vehicle",
    "skater_ground",
    "skater_dmo",
    "skater_vehicle",
    "solver_partition",
    "solver_contacts",
    "solver_joints_drives",
    "solver_iterations",
    "solver_feedback",
    "tick_terrain_release",
    "tick_terrain_focus",
    "tick_session_marker",
    "tick_queued_frame",
    "view_city_streaming",
    "view_map_frame",
    "view_character",
    "view_dmo",
    "view_vehicles",
    "view_pedestrians",
    "view_marker_ui",
    "terrain_poll",
    "terrain_admit",
    "terrain_finish",
    "terrain_depart",
    "terrain_request",
    "ai_manager",
    "ai_dispatch",
    "career",
    "lw2_los",
    "lw2_chase",
    "lw2_pedestrians",
    "lw2_vehicles",
    "lw2_dmo",
    "lw2_props",
];
pub const TICK_TERRAIN_RELEASE: usize = 26;
pub const TICK_TERRAIN_FOCUS: usize = 27;
pub const TICK_SESSION_MARKER: usize = 28;
pub const TICK_QUEUED_FRAME: usize = 29;
pub const VIEW_CITY_STREAMING: usize = 30;
pub const VIEW_MAP_FRAME: usize = 31;
pub const VIEW_CHARACTER: usize = 32;
pub const VIEW_DMO: usize = 33;
pub const VIEW_VEHICLES: usize = 34;
pub const VIEW_PEDESTRIANS: usize = 35;
pub const VIEW_MARKER_UI: usize = 36;
pub const TERRAIN_POLL: usize = 37;
pub const TERRAIN_ADMIT: usize = 38;
pub const TERRAIN_FINISH: usize = 39;
pub const TERRAIN_DEPART: usize = 40;
pub const TERRAIN_REQUEST: usize = 41;
pub const AI_MANAGER: usize = 42;
pub const AI_DISPATCH: usize = 43;
pub const CAREER: usize = 44;
pub const LW2_LOS: usize = 45;
pub const LW2_CHASE: usize = 46;
pub const LW2_PEDESTRIANS: usize = 47;
pub const LW2_VEHICLES: usize = 48;
pub const LW2_DMO: usize = 49;
pub const LW2_PROPS: usize = 50;
thread_local! { static TOTALS: RefCell<[Duration; COUNT]> = const { RefCell::new([Duration::ZERO; COUNT]) }; }
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SKATE2_FRAME_PROFILE").is_some())
}
pub fn record(index: usize, duration: Duration) {
    if enabled() {
        TOTALS.with(|totals| totals.borrow_mut()[index] += duration);
    }
}
pub fn take() -> [Duration; COUNT] {
    TOTALS.with(|totals| std::mem::replace(&mut *totals.borrow_mut(), [Duration::ZERO; COUNT]))
}
pub struct Span {
    index: usize,
    start: Instant,
}
pub fn span(index: usize) -> Option<Span> {
    enabled().then(|| Span {
        index,
        start: Instant::now(),
    })
}
impl Drop for Span {
    fn drop(&mut self) {
        record(self.index, self.start.elapsed());
    }
}
