//! Original S3 graph predicates. Producers and shared dispatch are external.
use skate3_core::animation::skeleton_input::name::encode;
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    BluntingBackslash,
    Approach { forwards: bool },
    TrickOutType { value: u32 },
    LandingIntoGrind,
    DroppingIn,
}
impl Condition {
    pub fn parse(a: &Attributes<'_>) -> Result<Option<Self>, String> {
        Ok(Some(match a.text("name") {
            Some("IsGrindBluntingBackslash") => Self::BluntingBackslash,
            Some("IsGrindApproach") => Self::Approach {
                forwards: encode(a.text("Facing").unwrap_or("F").as_bytes()) == encode(b"F"),
            },
            Some("GrindTrickOutTypeAllowed") => Self::TrickOutType {
                value: match a.text("type") {
                    Some("normal") => 0,
                    Some("left") => 1,
                    Some("right") => 2,
                    other => {
                        return Err(format!("Invalid GrindTrickOutTypeAllowed type: {other:?}"));
                    }
                },
            },
            Some("IsLandingIntoGrind") => Self::LandingIntoGrind,
            Some("IsDroppingIn") => Self::DroppingIn,
            _ => return Ok(None),
        }))
    }

    pub fn evaluate(self, p: &Physical) -> bool {
        match self {
            Self::BluntingBackslash => p.blunting_136 == 4, //82BBDC08
            Self::Approach { forwards } => {
                p.filtered_grinding_80 && p.approach_268 == u32::from(forwards)
            } //82BBDC80
            Self::TrickOutType { value } => p.trick_out_240 == value, //82BBDA10
            Self::LandingIntoGrind => p.air_grind_443 && p.air_time_184 < 0.2, //82BBDD70
            Self::DroppingIn => p.dropping_in_324,          //82BA41F0
        }
    }
}

/// Grinds bundle16 except filtered bundle64 byte80 / Air bundle8 fields.
/// No defaults or inference from contacts, physical state ID, grabs or airborne.
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    pub filtered_grinding_80: bool,
    pub blunting_136: u32,
    pub approach_268: u32,
    pub trick_out_240: u32,
    pub air_grind_443: bool,
    pub air_time_184: f32,
    pub dropping_in_324: bool,
}
