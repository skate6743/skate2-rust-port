//! ActionGraph condition leaves recovered from Skate 3 TU3.
//! Physical values are published by their real producers. An absent producer
//! is an integration error, never an implicit ground state or neutral speed.
use super::intents::IntentMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    None,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    GreaterAbsolute,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericCondition {
    pub comparison: Comparison,
    pub threshold: f32,
    pub absolute: bool,
}
impl NumericCondition {
    /// NumericCondition::Compare82C12AB8. The bge/ble branches test the
    /// absence of LT/GT, including unordered inputs; do not change to >=/<=.
    pub fn matches(self, value: f32) -> bool {
        let value = if self.absolute { value.abs() } else { value };
        match self.comparison {
            Comparison::None => false,
            Comparison::Equal => value == self.threshold,
            Comparison::NotEqual => value != self.threshold,
            Comparison::Greater => value > self.threshold,
            Comparison::Less => value < self.threshold,
            Comparison::GreaterEqual => !(value < self.threshold),
            Comparison::LessEqual => !(value > self.threshold),
            Comparison::GreaterAbsolute => value.abs() > self.threshold,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedInputs {
    /// PhysOut skateboard+164/+168/+192, respectively.
    pub speed: f32,
    pub forward_speed: f32,
    pub speed_and_slope: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalStateInputs {
    /// PhysOut filtered-state+0/+80/+8, respectively.
    pub category: u32,
    pub grinding: bool,
    pub grind_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PushBrakeInputs {
    /// Y of ground-output+80, used by82BA5150. Its producer must be supplied;
    /// this is not interchangeable with deck up or a selected contact normal.
    pub ground_axis_y: f32,
    /// PhysOut skeleton+598.
    pub skeleton_disables_push_brake: bool,
    /// Selected global collection316, layout+1844.
    pub maximum_ground_angle_degrees: f32,
}
impl PushBrakeInputs {
    /// Complete scalar result of DisablePushBrake82BA5150. The inverse-sine
    /// estimate retains the explicitly documented numerical validation boundary.
    pub fn disabled(self) -> bool {
        let degrees = crate::trigonometry::asin(self.ground_axis_y) * f32::from_bits(0x4265_2EE1);
        90.0 - degrees > self.maximum_ground_angle_degrees || self.skeleton_disables_push_brake
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConditionInputs {
    pub speeds: Option<SpeedInputs>,
    pub physical_state: Option<PhysicalStateInputs>,
    /// PhysOut miscellaneous+128, read by82BA15F8.
    pub time_since_last_input: Option<f32>,
    /// Actor stance accessors82BA4C30/82BA4C78, through component+1804.
    pub mirrored: Option<bool>,
    pub riding_fakie: Option<bool>,
    pub push_brake: Option<PushBrakeInputs>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionCondition {
    HasIntent {
        name: String,
        numeric: NumericCondition,
    },
    TimeSinceLastInput(NumericCondition),
    Speed {
        along_skate_z: bool,
        numeric: NumericCondition,
    },
    SpeedAndSlope(NumericCondition),
    FilteredState(u32),
    Grinding {
        name: Option<String>,
    },
    Mirrored,
    RidingFakie,
    DisablePushBrake,
    /// Name resolves relative to the containing state, using82C16C10. The
    /// host binds target once because the loaded hierarchy is immutable.
    CurrentState {
        name: String,
        target: Option<usize>,
    },
}
impl ActionCondition {
    pub fn evaluate(
        &self,
        input: &ConditionInputs,
        action_intents: &IntentMap,
        current: Option<usize>,
        parents: &[Option<usize>],
    ) -> Result<bool, &'static str> {
        Ok(match self {
            Self::HasIntent { name, numeric } => action_intents.get(name).is_some_and(|&value| {
                // HasAGIntent82BA0EA8 distinguishes presence-only from numeric.
                numeric.comparison == Comparison::None || numeric.matches(value)
            }),
            Self::TimeSinceLastInput(numeric) => numeric.matches(
                input
                    .time_since_last_input
                    .ok_or("TimeSinceLastInput needs PhysOut miscellaneous+128")?,
            ),
            Self::Speed {
                along_skate_z,
                numeric,
            } => {
                let speeds = input
                    .speeds
                    .ok_or("PhysicsSpeedCompare needs skateboard output")?;
                numeric.matches(if *along_skate_z {
                    speeds.forward_speed
                } else {
                    speeds.speed
                })
            }
            Self::SpeedAndSlope(numeric) => numeric.matches(
                input
                    .speeds
                    .ok_or("PhysicsSpeedAndSlopeCompare needs skateboard output+192")?
                    .speed_and_slope,
            ),
            Self::FilteredState(expected) => {
                input
                    .physical_state
                    .as_ref()
                    .ok_or("PhysFilteredState needs filtered physical state")?
                    .category
                    == *expected
            }
            Self::Grinding { name } => {
                let state = input
                    .physical_state
                    .as_ref()
                    .ok_or("IsGrinding needs filtered physical state")?;
                //824714D0 compares the five encoded words, including the
                //case/truncation semantics already used by skeleton attributes.
                state.grinding
                    && name.as_ref().is_none_or(|name| {
                        crate::animation::skeleton_input::name::encode(name.as_bytes())
                            == crate::animation::skeleton_input::name::encode(
                                state.grind_name.as_bytes(),
                            )
                    })
            }
            Self::Mirrored => input
                .mirrored
                .ok_or("IsMirrored needs the motion-graph stance owner")?,
            Self::RidingFakie => input
                .riding_fakie
                .ok_or("IsRidingFakie needs the motion-graph stance owner")?,
            Self::DisablePushBrake => input
                .push_brake
                .ok_or("DisablePushBrake needs ground/skeleton outputs and stock angle")?
                .disabled(),
            Self::CurrentState { target, .. } => {
                //82C13820 includes the current state itself, then its ancestors.
                let mut cursor = current;
                let mut matches = false;
                if let Some(target) = target {
                    while let Some(state) = cursor {
                        if state == *target {
                            matches = true;
                            break;
                        }
                        cursor = parents[state];
                    }
                }
                matches
            }
        })
    }
}

#[cfg(test)]
#[path = "tests/conditions.rs"]
mod tests;
