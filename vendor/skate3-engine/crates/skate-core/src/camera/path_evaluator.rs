//! Complete TU3 82E023D0/82E02490 orchestration. Backend adapters are external;
//! the trajectory request and moving-sphere provider are distinct native objects.

use super::{PathObstacle, PredictionPath, candidate_collision_time};

/// Native request vtable slots +4/+8/+12 (82DF9270/82DF9338/82DF9340).
/// A nonzero flag selects the manager acceleration vector in the stock adapter.
pub trait TrajectoryCollisionRequest {
    fn submit(&mut self, path: PredictionPath, context: u32, acceleration_flag: u8);
    fn is_ready(&mut self) -> bool;
    fn collision_time(&mut self) -> f32;
}

/// Native 82DF9400 provider. Implementations preserve native actor filtering and
/// sphere generation; the returned count must be at most the 50-record capacity.
pub trait MovingObstacleProvider {
    fn collect(
        &mut self,
        position: [f32; 4],
        velocity: [f32; 4],
        radius: f32,
        output: &mut [PathObstacle; 50],
    ) -> usize;
}

/// Fields touched by the two evaluator routines. The complete native constructor
/// and path setter own initialization; omitted native padding/fields are untouched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathEvaluator {
    /// Native word +8: only 0 and 1 have update behavior.
    pub request_state: u32,
    pub path: PredictionPath,
    /// Native byte +57, saved when submitting a request.
    pub acceleration_flag: u8,
    pub collision_time: f32,
    pub last_valid_time: f32,
    /// Native byte +72. Readiness counts even for a negative/no-hit result.
    pub found: u8,
    /// Native byte +73; only a ready asynchronous result replaces it.
    pub result_acceleration_flag: u8,
    /// Native byte +76.
    pub submitted_acceleration_flag: u8,
    /// Native word +80, compared as signed after wrapping increment.
    pub pending_polls: i32,
}

impl PathEvaluator {
    /// Complete 82E02490. Does not clear current-frame result fields on entry.
    pub fn update_request(&mut self, context: u32, request: &mut impl TrajectoryCollisionRequest) {
        match self.request_state {
            0 => {
                request.submit(self.path, context, self.acceleration_flag);
                self.submitted_acceleration_flag = self.acceleration_flag;
                self.request_state = 1;
                self.pending_polls = 0;
            }
            1 => {
                if request.is_ready() {
                    let result = request.collision_time();
                    let result = if result < 0.0 { f32::MAX } else { result };
                    self.found = 1;
                    self.request_state = 0;
                    self.result_acceleration_flag = self.submitted_acceleration_flag;
                    self.collision_time = select_time(self.collision_time, result);
                } else {
                    self.pending_polls = self.pending_polls.wrapping_add(1);
                    if self.pending_polls > 3 {
                        self.request_state = 0;
                        self.last_valid_time = f32::MAX;
                    }
                }
            }
            _ => {}
        }
    }

    /// Complete 82E023D0, including the native asynchronous-before-moving order.
    pub fn update(
        &mut self,
        context: u32,
        request: &mut impl TrajectoryCollisionRequest,
        moving: &mut impl MovingObstacleProvider,
    ) {
        self.found = 0;
        self.collision_time = f32::MAX;
        self.update_request(context, request);

        // Host scratch storage; only the provider's returned records are consumed.
        let mut obstacles = [PathObstacle {
            position: [0.0; 4],
            velocity: [0.0; 4],
            radius: 0.0,
        }; 50];
        let count = moving.collect(
            self.path.position,
            self.path.velocity,
            self.path.radius,
            &mut obstacles,
        );
        assert!(
            count <= obstacles.len(),
            "native moving-sphere capacity exceeded"
        );
        let time = candidate_collision_time(self.path, &obstacles[..count]);
        if time < self.path.horizon {
            self.found = 1;
            self.collision_time = select_time(self.collision_time, time);
        }
        if self.found != 0 {
            self.last_valid_time = self.collision_time;
        }
    }
}

// Preserve the native fsubs/fsel ordering (including unordered operands and ties).
fn select_time(current: f32, candidate: f32) -> f32 {
    if current - candidate >= 0.0 {
        candidate
    } else {
        current
    }
}
