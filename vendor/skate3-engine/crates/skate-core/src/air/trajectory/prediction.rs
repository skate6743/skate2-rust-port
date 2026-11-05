//! tTrajectory and complete PredictionResults8276C9D8/8276DBE0.
use super::math::{IDENTITY, STEP, Transform, UP, Vector, ZERO, reciprocal};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trajectory {
    pub position: Vector,     //0 metres
    pub velocity: Vector,     //16 metres/second
    pub acceleration: Vector, //32 metres/second squared
    pub duration: f32,        //48 seconds; -1 means unbounded
}
impl Trajectory {
    pub fn position_at(self, time: f32) -> Vector {
        let square = time * time;
        core::array::from_fn(|i| {
            (self.acceleration[i] * 0.5)
                .mul_add(square, self.velocity[i].mul_add(time, self.position[i]))
        })
    }
    pub fn velocity_at(self, time: f32) -> Vector {
        core::array::from_fn(|i| self.acceleration[i].mul_add(time, self.velocity[i]))
    }
    pub fn highest_position(self) -> (Vector, f32) {
        //82D2CCA0: downward velocity or nonnegative gravity has no apex.
        if self.velocity[1] < 0.0 || self.acceleration[1] >= 0.0 {
            (self.position, 0.0)
        } else {
            let t = self.velocity[1] * reciprocal(-self.acceleration[1]);
            (self.position_at(t), t)
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryRequest {
    pub trajectory: Trajectory, //request144..207
    pub radius: f32,            //208 metres
    pub start_error: f32,       //212 multiplier of radius
    pub end_error: f32,         //216 multiplier of radius
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryResult {
    pub contact_position: Vector,     //0
    pub contact_normal: Vector,       //16 first hit face normal
    pub landing_normal: Vector,       //32 velocity-qualified nearby-face average
    pub contact_time: f32,            //48 seconds; -1 is no contact
    pub contact_transform: Transform, //64 static world identity
    pub contact_frame: i32,           //128
    pub surface: u32,                 //132 original surface data
    pub geometry: u32,                //136 host geometry identity
}
impl QueryResult {
    pub const fn miss() -> Self {
        Self {
            contact_position: ZERO,
            contact_normal: UP,
            landing_normal: UP,
            contact_time: -1.0,
            contact_transform: IDENTITY,
            contact_frame: -1,
            surface: 0,
            geometry: 0,
        }
    }
    pub fn valid(self) -> bool {
        self.contact_time >= 0.0
    }
    ///PredictionResults::GetLandingNormal82D2D9E8.
    pub fn suggested_normal(self) -> Vector {
        if self.valid() {
            self.landing_normal
        } else {
            UP
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prediction {
    pub result: QueryResult,
    pub request: QueryRequest,
}
impl Prediction {
    pub fn collision_position(self) -> Vector {
        self.request
            .trajectory
            .position_at(self.result.contact_time)
    }
    pub fn collision_velocity(self) -> Vector {
        self.request
            .trajectory
            .velocity_at(self.result.contact_frame as f32 * STEP)
    }
}
