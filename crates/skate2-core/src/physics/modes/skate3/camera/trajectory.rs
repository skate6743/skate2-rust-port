//! Camera trajectory requests82DF9270/82DF9338/82DF9340. Rust executes the
//! world work synchronously; PathEvaluator still consumes it on its next poll.
use skate3_core::{
    camera::{PredictionPath, TrajectoryCollisionRequest, TrajectoryQuery},
    physics::board_world::BoardWorld,
};

#[derive(Clone, Debug)]
pub(super) struct TrajectoryResult {
    ready: bool,
    time: f32,
    pub error: Option<String>,
}
impl TrajectoryResult {
    pub fn new() -> Self { Self { ready: false, time: -1.0, error: None } }
}

pub(super) struct CameraTrajectory<'a> {
    pub world: &'a BoardWorld,
    pub gravity: [f32; 4],
    pub result: &'a mut TrajectoryResult,
}
impl TrajectoryCollisionRequest for CameraTrajectory<'_> {
    fn submit(&mut self, path: PredictionPath, _context: u32, acceleration_flag: u8) {
        self.result.ready = false;
        self.result.error = None;
        let query = TrajectoryQuery {
            position: path.position, velocity: path.velocity,
            gravity: if acceleration_flag != 0 { self.gravity } else { [0.0; 4] },
            duration: path.horizon, radius: path.radius,
            // 82DF92BC/82DF9304 set both maximum-error fractions to1.
            start_error: 1.0, end_error: 1.0,
        };
        match query.collision_time(|start, end, radius| world_line(self.world, start, end, radius)) {
            Ok(time) => { self.result.time = time; self.result.ready = true; }
            Err(error) => self.result.error = Some(error),
        }
    }
    fn is_ready(&mut self) -> bool { self.result.ready }
    fn collision_time(&mut self) -> f32 { self.result.time }
}

fn world_line(world: &BoardWorld, start: [f32; 4], end: [f32; 4], radius: f32)
    -> Result<Option<[f32; 4]>, String> {
    let hit = super::world_query::line(world, start, end, radius)?;
    Ok((hit.hit != 0).then_some(hit.position))
}
