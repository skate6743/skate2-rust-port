//! Normal camera subject accessors and reference points, TU3 82DF69C0/82DF78A8.
//! Physical and animation producers supply this snapshot together, after their
//! tick. No grounded state, bone position, or camera anchor is inferred here.
use super::{
    AnchorTrackingSubject, AvoidanceSubject, ReferenceHeightSubject, RigModeSubject,
    RigPositioningSubject,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Subject {
    /// 82DF80D8 -> setter24. This is the native camera subject basis.
    pub transform: [[f32; 4]; 4],
    /// Physical output bundle5+432 -> setter32.
    pub skeleton_root: [[f32; 4]; 4],
    /// Physical output bundle5+64 -> setter92, getter444.
    pub hips_position: [f32; 4],
    /// Physical output bundle8+96 -> setter100, getter452.
    pub last_valid_ground_up: [f32; 4],
    pub reference_positions: [[f32; 4]; 10],
    pub context: u32,
    /// Physical output bundle8+268 -> setter124, getter476.
    pub pumping_acceleration: f32,
    /// Physical output bundle7+32 -> setter172, getter524.
    pub state_height_32: f32,
    pub in_ground_physics: u8,
    pub grinding: u8,
    pub trajectory_valid: u8,
    pub wiping_out: u8,
    pub physically_pushing: u8,
    pub at_pushable_speed: u8,
    pub off_board: u8,
    /// Bundle2 byte452 -> setter304, getter656. Its producer owns the meaning.
    pub air_flag_452: u8,
    /// Bundle7 byte81 -> setter308, getter660.
    pub state_flag_81: u8,
    /// Bundle15 float200 > 0 and flags204 bit2 -> setter316, getter668.
    pub broken_bone_slowmo: u8,
    /// Setter328 is cleared by82DF69C0; later subject writers remain explicit.
    pub subject_flag_328: u8,
}

/// Exact inputs used by82DF78A8; names identify their role in the camera's
/// reference-point table, while comments retain the physical output binding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferencePointInputs {
    pub head: [f32; 4],                  // skeleton+32
    pub hips: [f32; 4],                  // skeleton+64
    pub left_foot: [f32; 4],             // skeleton+144
    pub right_foot: [f32; 4],            // skeleton+160
    pub board: [f32; 4],                 // skeleton+480
    pub centre_of_mass: [f32; 4],        // bundle9+64
    pub damped_centre_of_mass: [f32; 4], // subject464 (bundle9+80)
    pub grind_position: [f32; 4],        // bundle4+96
    pub tracked_anchor: [f32; 4],        // CameraMan604 -> Rig184 -> tracker16
    pub incline_normal: [f32; 4],        // CameraMan784
    pub grinding: u8,
}

impl ReferencePointInputs {
    /// The stock list has ten entries. The collision positioner requests
    /// entries3,0,7, so substituting one board point for this list changes it.
    pub fn positions(self) -> [[f32; 4]; 10] {
        let feet = core::array::from_fn(|i| (self.left_foot[i] + self.right_foot[i]) * 0.5);
        [
            self.head,
            self.hips,
            if self.grinding != 0 {
                self.grind_position
            } else {
                self.board
            },
            self.centre_of_mass,
            self.board,
            self.centre_of_mass,
            self.tracked_anchor,
            feet,
            self.damped_centre_of_mass,
            self.incline_normal,
        ]
    }
}

impl RigModeSubject for Subject {
    fn flag_568(&mut self) -> u8 {
        self.in_ground_physics
    }
    fn flag_596(&mut self) -> u8 {
        self.grinding
    }
    fn flag_680(&mut self) -> u8 {
        self.subject_flag_328
    }
    fn flag_540(&mut self) -> u8 {
        self.trajectory_valid
    }
}
impl AvoidanceSubject for Subject {
    fn transform_376(&mut self) -> [[f32; 4]; 4] {
        self.transform
    }
    fn vector_452(&mut self) -> [f32; 4] {
        self.last_valid_ground_up
    }
    fn vector_444(&mut self) -> [f32; 4] {
        self.hips_position
    }
    fn context_532(&mut self) -> u32 {
        self.context
    }
    fn flag_608(&mut self) -> u8 {
        self.off_board
    }
    fn flag_548(&mut self) -> u8 {
        self.wiping_out
    }
}
impl AnchorTrackingSubject for Subject {
    fn flag_584(&mut self) -> u8 {
        self.physically_pushing
    }
    fn flag_588(&mut self) -> u8 {
        self.at_pushable_speed
    }
    fn value_476(&mut self) -> f32 {
        self.pumping_acceleration
    }
    fn flag_548(&mut self) -> u8 {
        self.wiping_out
    }
}
impl ReferenceHeightSubject for Subject {
    fn flag_656(&mut self) -> u8 {
        self.air_flag_452
    }
    fn flag_608(&mut self) -> u8 {
        self.off_board
    }
}
impl RigPositioningSubject for Subject {
    fn flag_668(&mut self) -> u8 {
        self.broken_bone_slowmo
    }
    fn flag_548(&mut self) -> u8 {
        self.wiping_out
    }
    fn flag_660(&mut self) -> u8 {
        self.state_flag_81
    }
    fn value_524(&mut self) -> f32 {
        self.state_height_32
    }
    fn transform_384(&mut self) -> [[f32; 4]; 4] {
        self.skeleton_root
    }
    fn position_360(&mut self, index: u32) -> [f32; 4] {
        self.reference_positions[index as usize]
    }
    fn context_532(&mut self) -> u32 {
        self.context
    }
}
