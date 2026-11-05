//! Complete candidate-list evaluator TU3 82E02608, used by avoidance82E023D0.
//! Query enumeration and its separate asynchronous world provider are upstream.

/// Fields consumed from each native 48-byte candidate; remaining words are not
/// interpreted by this evaluator. Velocity is in the same world frame as path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathObstacle {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PredictionPath {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub radius: f32,
    pub horizon: f32,
}

/// Returns the earliest accepted closest-approach time, or native FLT_MAX.
/// Native does not solve first surface entry time; tangent cases are rejected.
pub fn candidate_collision_time(path: PredictionPath, obstacles: &[PathObstacle]) -> f32 {
    let mut result = f32::MAX;
    for obstacle in obstacles {
        let separation = core::array::from_fn(|i| obstacle.position[i] - path.position[i]);
        let relative_velocity = core::array::from_fn(|i| obstacle.velocity[i] - path.velocity[i]);
        let radius = obstacle.radius + path.radius;
        let clearance = super::vector_tracker::dot(separation, separation) - radius * radius;
        let time = if clearance < 0.0 {
            0.0
        } else {
            let approach = super::vector_tracker::dot(separation, relative_velocity);
            if !(approach < 0.0) {
                continue;
            }
            let speed_squared = super::vector_tracker::dot(relative_velocity, relative_velocity);
            let time = (-1.0 / speed_squared) * approach;
            if !(approach.mul_add(time, clearance) < 0.0) {
                continue;
            }
            time
        };
        if time < path.horizon && time < result {
            result = time;
        }
    }
    result
}
