//! Typed bindings for the remaining stock MotionGraph gameplay operations.
//!
//! These operations are deliberately represented as authored gameplay nodes,
//! not compatibility no-ops. Their producers are completed incrementally and
//! execution fails with the exact authored operation until that producer is
//! published by the corresponding physics owner.
//!
//! Grind attributes, crouch control and fade belong exclusively to
//! `motion_grind`; this fallback must not claim those authored handlers.
use skate_data::state_graph::attributes::Attributes;
use skate3_core::animation::output::attributes::AttributeName;
use skate3_core::animation::skeleton_input::name::encode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrabType {
    Fs,
    Bs,
    Nose,
    Tail,
}

impl GrabType {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "FS" => Ok(Self::Fs),
            "BS" => Ok(Self::Bs),
            "Nose" => Ok(Self::Nose),
            "Tail" => Ok(Self::Tail),
            other => Err(format!("SetGrabType has unknown grab type {other:?}")),
        }
    }
    pub fn as_value(self) -> f32 {
        // TU3 82BB9BD0: zero is no grab; authored types are one through four.
        match self { Self::Fs => 1.0, Self::Bs => 2.0, Self::Nose => 3.0, Self::Tail => 4.0 }
    }
}

/// TU3 82BBF180..F2EC, constants at 820DDEA8..DEE4. Sector windows
/// expand when neighboring variants are absent; first matching window wins.
pub(super) fn select_grab_score<'a>(
    base: &'a str,
    directions: [Option<&'a str>; 4],
    x: f32,
    y: f32,
) -> &'a str {
    let [up, left, down, right] = directions;
    if (x * x + y * y).sqrt() <= 0.5 {
        return base;
    }
    let angle = y.atan2(x).rem_euclid(std::f32::consts::TAU);
    let windows = [
        (up, if right.is_some() { 44.0 } else { 359.0 }, if left.is_some() { 134.0 } else { 181.0 }),
        (left, if up.is_some() { 44.0 } else { 89.0 }, if down.is_some() { 226.0 } else { 271.0 }),
        (down, if left.is_some() { 224.0 } else { 179.0 }, if right.is_some() { 316.0 } else { 1.0 }),
        (right, if down.is_some() { 314.0 } else { 269.0 }, if up.is_some() { 46.0 } else { 91.0 }),
    ];
    for (name, start, end) in windows {
        let start = start * (std::f32::consts::PI / 180.0);
        let end = end * (std::f32::consts::PI / 180.0);
        let inside = if end < start { angle >= start || angle <= end }
            else { angle >= start && angle <= end };
        if inside && let Some(name) = name { return name; }
    }
    base
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    MatchAirTime,
    UpdateStandingOnCar,
    InitMovingObjects,
    MovingObject(super::motion_moving_object::Behaviour),
    SetGrabType { grab: String },
    JumpInto { attribute: AttributeName },
    ScoringHandPlants,
    SetHandPlantAnticLength,
    ScoringGrabs {
        grab_name: String,
        up: Option<String>,
        left: Option<String>,
        down: Option<String>,
        right: Option<String>,
        intent_x: String,
        intent_y: String,
        invert_y: bool,
        polar: bool,
    },
    AirDismounting,
    TweakProject {
        intent_x: String,
        intent_y: String,
        attribute_x: AttributeName,
        attribute_y: AttributeName,
    },
    FootPlantAbsorb,
    FingerFlipOut { grab_intent: String },
    EnterSkitchingBehaviour,
    SkitchingBehaviour,
    /// Ctor 0x82BCAF40: "direction" right is 2, left 1, otherwise 0.
    SkitchShimmyingBehaviour { direction: u8 },
    SetManualAngle { attribute: AttributeName },
    HippyJumpAntic,
    LandOnBoard,
}

impl Operation {
    pub fn recognizes(name: &str) -> bool {
        matches!(
            name,
            "MatchAirTime"
                | "UpdateStandingOnCar"
                | "InitMovingObjects"
                | "MovingObject"
                | "SetGrabType"
                | "JumpInto" | "ScoringHandPlants" | "SetHandPlantAnticLength"
                | "ScoringGrabs" | "AirDismounting" | "TweakProject"
                | "FootPlantAbsorb" | "FingerFlipOut" | "EnterSkitchingBehaviour"
                | "SkitchingBehaviour" | "SkitchShimmyingBehaviour"
                | "SetManualAngle" | "HippyJumpAntic"
                | "LandOnBoard"
        )
    }

    pub fn parse(a: &Attributes<'_>) -> Self {
        match a.text("name").unwrap_or("") {
            "MatchAirTime" => Self::MatchAirTime,
            "UpdateStandingOnCar" => Self::UpdateStandingOnCar,
            "InitMovingObjects" => Self::InitMovingObjects,
            "MovingObject" => Self::MovingObject(super::motion_moving_object::Behaviour::parse(a)),
            "SetGrabType" => Self::SetGrabType { grab: a.text("grab").unwrap_or("").to_owned() },
            "JumpInto" => Self::JumpInto {
                attribute: encode(
                    a.text("attribute")
                        .or_else(|| a.text("attr"))
                        .unwrap_or("JumpInto")
                        .as_bytes(),
                ),
            }, "ScoringHandPlants" => Self::ScoringHandPlants,
            "SetHandPlantAnticLength" => Self::SetHandPlantAnticLength,
            "ScoringGrabs" => Self::ScoringGrabs {
                grab_name: required_text(a, "grabName"),
                up: a.text("up").map(str::to_owned),
                left: a.text("left").map(str::to_owned),
                down: a.text("down").map(str::to_owned),
                right: a.text("right").map(str::to_owned),
                intent_x: a.text("intentX").or_else(|| a.text("angle")).unwrap_or("").to_owned(),
                intent_y: a.text("intentY").or_else(|| a.text("magnitude")).unwrap_or("").to_owned(),
                invert_y: a.boolean_byte("invertY", 0) != 0,
                polar: a.text("intentX").is_none(),
            }, "AirDismounting" => Self::AirDismounting,
            "TweakProject" => Self::TweakProject {
                // TU3 factory 82BC9510 binds these names directly.
                intent_x: "TweakX".to_owned(),
                intent_y: "TweakY".to_owned(),
                attribute_x: encode(b"tweak_x"),
                attribute_y: encode(b"tweak_y"),
            }, "FootPlantAbsorb" => Self::FootPlantAbsorb,
            "FingerFlipOut" => Self::FingerFlipOut {
                //Factory82BC7C18 binds authored grabintent and literal Grabbing.
                grab_intent: required_text(a, "grabintent"),
            },
            "EnterSkitchingBehaviour" => Self::EnterSkitchingBehaviour,
            "SkitchingBehaviour" => Self::SkitchingBehaviour,
            "SkitchShimmyingBehaviour" => Self::SkitchShimmyingBehaviour {
                direction: match a.text("direction") {
                    Some("right") => 2,
                    Some("left") => 1,
                    _ => 0,
                },
            },
            "SetManualAngle" => Self::SetManualAngle {
                // Factory82BC9678/84 uses literal8231D5C8, no authored override.
                attribute: encode(b"manual_angle"),
            },
            "HippyJumpAntic" => Self::HippyJumpAntic, "LandOnBoard" => Self::LandOnBoard,
            _ => unreachable!("unrecognized stock gameplay operation"),
        }
    }
}

fn required_text(a: &Attributes<'_>, name: &str) -> String {
    a.text(name).unwrap_or("").to_owned()
}
