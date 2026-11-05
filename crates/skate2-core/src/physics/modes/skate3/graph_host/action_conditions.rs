//! Original ActionGraph leaves that consume the prior animation/state output.
use super::{action::ActionHost, motion_gameplay_conditions::GameplayCondition};
use skate3_core::{
    animation::{output::attributes::AttributeName, skeleton_input::name::encode},
    graph::{conditions::NumericCondition, controller::Frame},
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum ActionCondition {
    Gesture(crate::skate3_native::input::gesture_catalog::Group),
    AnimationAttribute {
        name: AttributeName,
        sequence_id: i32,
        numeric: NumericCondition,
    },
    ParentTime {
        target: Option<usize>,
        numeric: NumericCondition,
    },
    Physical(GameplayCondition),
    Tricking,
    DroppingIn,
    InLocomotion,
    DisableDismount,
    TimeToLand(NumericCondition),
}
impl ActionCondition {
    pub fn parse(a: &Attributes<'_>) -> Result<Option<Self>, String> {
        Ok(Some(match a.text("name").unwrap_or("") {
            "HasGestureIntent" => Self::Gesture(crate::skate3_native::input::gesture_catalog::Group::parse(
                a.text("group").unwrap_or(""),
            )?),
            "HasAnimAttribute" => Self::AnimationAttribute {
                name: encode(a.text("attribute").unwrap_or("").as_bytes()),
                sequence_id: f32::from_bits(a.float_bits("sequenceid", (-1.0f32).to_bits())) as i32,
                numeric: super::condition_nodes::numeric(a),
            },
            "InParentStateForTime" => Self::ParentTime {
                target: None,
                numeric: super::condition_nodes::numeric(a),
            },
            "IsGrabbingObject" | "IsHandPlanting" | "IsFootPlanting" => {
                Self::Physical(GameplayCondition::parse(a)?)
            }
            "IsDroppingIn" => Self::DroppingIn,
            "IsTricking" => Self::Tricking,
            "IsInLocomotion" => Self::InLocomotion,
            "DisableDismount" => Self::DisableDismount,
            "TimeToLand" => Self::TimeToLand(super::condition_nodes::numeric(a)),
            _ => return Ok(None),
        }))
    }
    pub fn evaluate(&self, host: &ActionHost, frame: &Frame) -> Result<bool, String> {
        Ok(match self {
            Self::Gesture(group) => group.has_intent(|name| host.action_intents.contains_key(name)),
            Self::AnimationAttribute {
                name,
                sequence_id,
                numeric,
            } => super::condition_nodes::animation_attribute(
                &host.animation_attributes,
                *name,
                *sequence_id,
                *numeric,
            ),
            Self::ParentTime { target, numeric } => {
                super::condition_nodes::state_time(frame, *target, *numeric)
            }
            Self::Physical(condition) => condition
                .evaluate_physical(
                    host.gameplay_conditions
                        .as_ref()
                        .ok_or("ActionGraph requires actual physical condition outputs")?,
                )
                .ok_or("Condition is not a physical-only original leaf")?,
            //82BA56B8 uses specific MotionGraph virtual100=8258F8D8 bit27.
            Self::DroppingIn => host.dropping_in.ok_or("IsDroppingIn requires completed grind output")?,
            Self::Tricking => host
                .is_tricking
                .ok_or("IsTricking requires actual MotionGraph flag27")?,
            //Factory82BC3F68 installs8231ED5C; virtual48=8274CA90 is li r3,0;blr.
            Self::InLocomotion => false,
            Self::DisableDismount => super::motion_dismount::Condition
                .evaluate(host.condition_inputs.push_brake.as_ref())?,
            Self::TimeToLand(numeric) => {
                let p = host.gameplay_conditions.as_ref()
                    .ok_or("ActionGraph TimeToLand requires actual physical condition outputs")?;
                p.time_to_land_valid && numeric.matches(p.time_to_land)
            }
        })
    }
}
