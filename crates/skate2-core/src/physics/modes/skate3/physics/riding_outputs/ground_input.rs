//! Snapshot inputs copied by original TU382D8E5C0 for the Ground job.
use skate3_core::{math::Vector3, player::input_phase::ProcessedPhysicsInput};

pub(super) struct GroundPacketInputs {
    pub wheel_normal: Vector3,
    pub dynamic_up: Vector3,
    pub speed: f32,
    pub absolute_speed: f32,
    pub wheel_count: i32,
}

impl GroundPacketInputs {
    pub fn from_processed(input: &ProcessedPhysicsInput) -> Self {
        let vector = |bits: [u32; 4]| {
            let value = bits.map(f32::from_bits);
            Vector3::new(value[0], value[1], value[2])
        };
        Self {
            //82D8E790: Processed464 -> input720.
            wheel_normal: vector(input.vectors_464_480_496_512_528[0]),
            //82D8E79C: Processed528 -> input736. Board112 is a retained
            //acceleration-derived normal, NOT CollisionInfo's overall normal.
            dynamic_up: vector(input.vectors_464_480_496_512_528[4]),
            speed: input.scalar_2652,          //82D8E7C0 -> input772.
            absolute_speed: input.scalar_2616, //82D8E7D8 -> input780.
            wheel_count: input.wheel_count_2556 as i32, //82D8E924 -> input804.
        }
    }
}

#[cfg(any())]
#[path = "ground_input_tests.rs"]
mod tests;
