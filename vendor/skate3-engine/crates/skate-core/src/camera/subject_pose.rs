//! Pose publication82DF80D8 and82DF7558..82DF76B4. Physical outputs remain
//! authoritative; this owns only the camera's proven one-frame pose history.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubjectPoseInputs {
    pub physical_transform: [[f32; 4]; 4],
    pub skeleton_root: [[f32; 4]; 4],
    pub center_of_mass: [f32; 4],
    pub reckoned_center_of_mass: [f32; 4],
    /// State output7 bytes59 and75 select the skeleton root.
    pub wiping_out: bool,
    pub state_flag_75: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PublishedSubjectPose {
    pub transform: [[f32; 4]; 4],
    pub damped_center_of_mass: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubjectPosePublisher {
    previous_physical_transform: [[f32; 4]; 4],
}

impl SubjectPosePublisher {
    pub fn new() -> Self {
        // Subject constructor82E07C08 initializes its three transform matrices
        // from the native unit-axis constants, with a zero translation vector.
        Self {
            previous_physical_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0; 4],
            ],
        }
    }

    pub fn publish(&mut self, input: SubjectPoseInputs) -> PublishedSubjectPose {
        let transform = if input.wiping_out || input.state_flag_75 {
            input.skeleton_root
        } else {
            self.previous_physical_transform
        };
        // 82DF80D8 executes before publisher setter28, preserving that delay.
        self.previous_physical_transform = input.physical_transform;
        let mut damped = input.reckoned_center_of_mass;
        let delta = core::array::from_fn(|i| damped[i] - input.center_of_mass[i]);
        let distance = super::vector_tracker::length(delta);
        let maximum = f32::from_bits(0x3e147ae1);
        if distance > maximum {
            let inverse = super::vector_tracker::refined_reciprocal(distance);
            damped = core::array::from_fn(|i| {
                (inverse * delta[i]).mul_add(maximum, input.center_of_mass[i])
            });
        }
        PublishedSubjectPose {
            transform,
            damped_center_of_mass: damped,
        }
    }
}
