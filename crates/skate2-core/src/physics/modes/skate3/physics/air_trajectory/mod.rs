//! Real stock-settings and BoardWorld adapter for the player trajectory selector.
mod grind;
mod settings;
use crate::skate3_native::grind_world::StaticProvider;
pub(crate) use grind::GrindContext;
use std::sync::Arc;
mod world;
use skate3_core::{
    air::trajectory::{
        LaunchInfo, QueryRequest, QueryResult, SelectorInput, SelectorSettings, TrajectorySelector,
        query_trajectory,
    },
    physics::board_world::BoardWorld,
};
use skate_data::collections::Collections;

pub struct AirTrajectoryRuntime {
    pub selector: TrajectorySelector,
    pub settings: SelectorSettings,
    pending_results: Option<Vec<QueryResult>>,
    /// The physics tick now running and the one whose batch is pending. TU3's predictor queries
    /// resolve in the world step after their launch (Update82D68800 waits on mFutureResult), so a
    /// batch completes on a later tick, never the tick that submitted it.
    tick: u64,
    submitted_tick: u64,
    grind_settings: grind::Settings,
    grind_world: Option<Arc<StaticProvider>>,
    nearby_grinds: Vec<usize>,
}
impl AirTrajectoryRuntime {
    ///Full82E099A0 query shared by trajectory and Footplant callers.
    pub fn query(world: &BoardWorld, request: QueryRequest) -> Result<QueryResult, String> {
        query_trajectory(
            request,
            |start, end, radius| world::line(world, start, end, radius),
            |position, radius| world::nearby(world, position, radius),
        )
    }
    pub fn load(collections: &Collections) -> Result<Self, String> {
        Ok(Self {
            selector: TrajectorySelector::new(),
            settings: {
                let mut settings = settings::load(collections)?;
                // SKATE2_DIAGNOSTIC_TRAJECTORY_MAX_TIME: a shorter landing query, so ordinary
                // launches take the no-landing air.
                if let Some(seconds) = std::env::var("SKATE2_DIAGNOSTIC_TRAJECTORY_MAX_TIME").ok().and_then(|v| v.parse().ok()) {
                    settings.trajectory_max_time = seconds;
                }
                settings
            },
            pending_results: None,
            tick: 0,
            submitted_tick: 0,
            grind_settings: grind::Settings::load(collections)?,
            grind_world: None,
            nearby_grinds: Vec::new(),
        })
    }
    pub fn launch(
        &mut self,
        info: LaunchInfo,
        input: SelectorInput,
        world: &BoardWorld,
    ) -> Result<bool, String> {
        let requested = info.start_velocity;
        let launched = self.selector.launch(info, input, &self.settings)?;
        if trace_launch() {
            eprintln!(
                "skate3 launch: requested={requested:?} adjusted={:?} player_jumped={} max_time={} max_drop={}",
                self.selector.launch_info().map(|i| i.start_velocity),
                info.player_jumped,
                self.settings.trajectory_max_time,
                self.settings.trajectory_max_drop,
            );
        }
        if launched {
            self.submit(world)?;
        }
        Ok(launched)
    }
    /// The physics tick about to run.
    pub fn begin_tick(&mut self, tick: u64) {
        self.tick = tick;
    }
    pub fn bind_grind_world(&mut self, provider: Arc<StaticProvider>) {
        self.grind_world = Some(provider);
        self.nearby_grinds.clear();
    }
    pub fn update(
        &mut self,
        input: SelectorInput,
        world: &BoardWorld,
        context: GrindContext,
    ) -> Result<bool, String> {
        if self.pending_results.is_none() || self.submitted_tick == self.tick {
            return Ok(self.selector.update_without_completion());
        }
        let results = self.pending_results.take().expect("checked above");
        let provider = self
            .grind_world
            .as_deref()
            .ok_or("Trajectory static grind provider was not registered")?;
        let trace = trace_launch();
        let candidates: Vec<String> = if trace {
            self.selector.requests().iter().zip(&results).map(|(request, result)| format!(
                "v=[{:.2},{:.2},{:.2}] duration={:.2} contact={:.2}",
                request.trajectory.velocity[0], request.trajectory.velocity[1], request.trajectory.velocity[2],
                request.trajectory.duration, result.contact_time,
            )).collect()
        } else {
            Vec::new()
        };
        let valid = self.selector.complete_batch(
            &results,
            input,
            &self.settings,
            |prediction, acquire| {
                self.grind_settings.evaluate(
                    prediction,
                    acquire,
                    world,
                    provider,
                    &mut self.nearby_grinds,
                    input.grind_lock_distance,
                    context,
                )
            },
            |start, end, radius| world::line(world, start, end, radius),
        )?;
        if trace {
            let selection = self.selector.selection();
            eprintln!(
                "skate3 launch candidates: valid={valid} winner={:?} winner_contact={:?} wall_ride={:?} grind={} [{}]",
                selection.map(|s| s.candidate_index),
                selection.map(|s| s.prediction.result.contact_time),
                selection.map(|s| s.wall_ride),
                selection.is_some_and(|s| s.grind.is_some()),
                candidates.join("; "),
            );
        }
        if self.selector.pending() {
            self.submit(world)?;
        }
        Ok(valid)
    }
    fn submit(&mut self, world: &BoardWorld) -> Result<(), String> {
        let results = self
            .selector
            .requests()
            .iter()
            .map(|&request| Self::query(world, request))
            .collect::<Result<Vec<_>, _>>()?;
        self.pending_results = Some(results);
        self.submitted_tick = self.tick;
        Ok(())
    }
}

/// SKATE2_TRACE_SKATE3_LAUNCH: each launch's velocity and its candidates' flights and winner.
fn trace_launch() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_LAUNCH").is_some())
}
