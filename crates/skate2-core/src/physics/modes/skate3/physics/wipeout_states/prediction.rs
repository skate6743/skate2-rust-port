//! Original Wipeout Update82D3C02C..C288; completed query precedes next submit.
use crate::skate3_native::physics::air_trajectory::AirTrajectoryRuntime;
use skate3_core::{
    air::trajectory::{QueryRequest, QueryResult, Trajectory},
    physics::board_world::BoardWorld,
    player::wipeout_state::State,
};

pub(crate) struct Prediction {
    pub result: QueryResult,
    pending: Option<(QueryResult, Trajectory, bool)>,
}
impl Prediction {
    pub fn new() -> Self {
        Self {
            result: QueryResult::miss(),
            pending: None,
        }
    }
    ///Enter clears completion816; the old results are not consumed afterward.
    pub fn reset(&mut self) {
        self.pending = None;
        self.result = QueryResult::miss();
    }

    pub fn advance(
        &mut self,
        state: &mut State,
        world: &BoardWorld,
        com_position: [f32; 4],
        gravity: [f32; 4],
    ) -> Result<(), String> {
        if let Some((result, trajectory, surface_query)) = self.pending.take() {
            self.result = result;
            if surface_query {
                if !result.valid() || result.surface & 0xF80 != 0x600 {
                    state.below_surface = false;
                    state.special_surface = false;
                }
            } else if result.valid() {
                state.predicted_time = result.contact_time;
                state.imminent_surface_twelve =
                    result.surface & 0xF80 == 0x600 && result.contact_time < 0.1;
                state.predicted_position = trajectory.position_at(result.contact_time);
            } else {
                state.predicted_position = trajectory.position_at(3.0);
                //The native miss branch preserves832 and573.
            }
        }
        let surface_query = state.below_surface;
        state.surface_query = surface_query;
        let request = if surface_query {
            let mut position = com_position;
            position[1] = state.surface_height + 0.1;
            QueryRequest {
                trajectory: Trajectory {
                    position,
                    velocity: [0.0, -0.2, 0.0, 0.0],
                    acceleration: [0.0; 4],
                    duration: 1.0,
                },
                radius: 0.01,
                start_error: 1.0,
                end_error: 1.0,
            }
        } else {
            QueryRequest {
                trajectory: Trajectory {
                    position: com_position,
                    velocity: state.velocity,
                    acceleration: gravity,
                    duration: 3.0,
                },
                radius: 0.3,
                start_error: 1.0,
                end_error: 1.0,
            }
        };
        let result = AirTrajectoryRuntime::query(world, request)?;
        //Host query execution is synchronous. The next tick's early control
        //must see this completed result, before consuming it at the update tail.
        self.result = result;
        self.pending = Some((result, request.trajectory, surface_query));
        Ok(())
    }
}
