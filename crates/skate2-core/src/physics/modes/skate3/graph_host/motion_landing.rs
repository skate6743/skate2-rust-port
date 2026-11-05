//! Original TU3 landing graph configuration and persistent instance state.
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    IsAnticipating,
    IsLanding,
    IsManualing,
    IsDoingTrick,
    SetLandingData,
    DisableTricks { length: f32 },
    ChooseRandomLanding { count: i32 },
}
impl Operation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        Some(match a.text("name")? {
            "IsAnticipating" => Self::IsAnticipating,
            "IsLanding" => Self::IsLanding,
            "IsManualing" => Self::IsManualing,
            "IsDoingTrick" => Self::IsDoingTrick,
            //82BC7618 fixes this operation's attribute to disttocog.
            "SetLandingData" => Self::SetLandingData,
            "DisableTricks" => Self::DisableTricks {
                length: a
                    .get("length")
                    .map_or(0.5, |v| f32::from_bits(v.float_bits)),
            },
            "ChooseRandomLanding" => Self::ChooseRandomLanding {
                //82BC7788 truncates the configured float toward zero.
                count: a
                    .get("numlandings")
                    .map_or(1.0, |v| f32::from_bits(v.float_bits)) as i32,
            },
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    HasLandingType(u32),
    HasTiltToLargeForPreland,
    AllowedToTrick,
}
impl Condition {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        Some(match a.text("name")? {
            "HasLandingType" => {
                Self::HasLandingType(match a.text("landingType").unwrap_or("straight") {
                    "spin" => 2,
                    "sketchy" => 1,
                    _ => 0,
                })
            }
            "HasTiltToLargeForPreland" => Self::HasTiltToLargeForPreland,
            "AllowedToTrick" => Self::AllowedToTrick,
            _ => return None,
        })
    }
    pub fn evaluate(
        self,
        physical: Option<Physical>,
        flags: &Flags,
        override_preland: Option<bool>,
    ) -> Result<bool, String> {
        match self {
            Self::HasLandingType(kind) => {
                Ok(physical.ok_or("HasLandingType requires Animation96")?.kind == kind)
            }
            Self::HasTiltToLargeForPreland => override_preland
                .ok_or("HasTiltToLargeForPreland requires original prelanding output".into()),
            //82BC5860 ->8231F460+48 ->82BA74D8 ->specific v116.
            Self::AllowedToTrick => Ok(flags.tricks_allowed),
        }
    }
}

/// Original MotionGraph CA4 flags, distinct from physical balance mode.
pub struct Flags {
    pub anticipating: bool,
    pub landing: bool,
    pub manualing: bool,
    pub doing_trick: bool,
    pub tricks_allowed: bool,
}
impl Default for Flags {
    fn default() -> Self {
        //825953B0 clears bits30..27 and sets bit25.
        Self {
            anticipating: false,
            landing: false,
            manualing: false,
            doing_trick: false,
            tricks_allowed: true,
        }
    }
}

/// Completed physical conditioner publication; no inferred landing classification.
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    pub height: f32,
    pub spin: f32,
    pub kind: u32,
    pub last_good_landing_velocity: f32,
}

/// Original instance scalar8 (height or countdown) and byte12 (timer complete).
#[derive(Default)]
pub struct State {
    pub(super) value: f32,
    pub(super) complete: bool,
}
