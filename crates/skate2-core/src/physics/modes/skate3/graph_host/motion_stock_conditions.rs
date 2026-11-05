//! Recovered stock predicates using completed physical publications.
//! Unsupported archive guesses are deliberately left to the graph's diagnostic path.
use super::motion::MotionHost;
use skate3_core::graph::conditions::NumericCondition;
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    CanLandOnBoard,
    DistToEdge(NumericCondition),
    IsDeckFree,
    IsBipedCommittedToMotion,
    EnoughDistToObstacle { database: String, animation: String },
    CanBipedLand,
    IsCrouchedEnoughForBlendToGrabCycle,
    TrucksOrDeckInContact,
    PhysicsWantsManualExit,
    ApexReached,
}
impl Condition {
    pub fn recognizes(name: &str) -> bool {
        matches!(name, "CanLandOnBoard" | "DistToEdge" | "IsDeckFree" |
            "IsBipedCommittedToMotion" | "EnoughDistToObstacle" | "CanBipedLand" |
            "IsCrouchedEnoughForBlendToGrabCycle" | "TrucksOrDeckInContact" |
            "PhysicsWantsManualExit" | "ApexReached")
    }
    pub fn parse(a: &Attributes<'_>) -> Self {
        match a.text("name").unwrap_or("") {
            "CanLandOnBoard" => Self::CanLandOnBoard,
            "DistToEdge" => Self::DistToEdge(super::condition_nodes::numeric(a)),
            "IsDeckFree" => Self::IsDeckFree,
            "IsBipedCommittedToMotion" => Self::IsBipedCommittedToMotion,
            "EnoughDistToObstacle" => Self::EnoughDistToObstacle {
                database: a.text("db").unwrap_or("").to_owned(),
                animation: a.text("anim").unwrap_or("").to_owned(),
            },
            "CanBipedLand" => Self::CanBipedLand,
            "IsCrouchedEnoughForBlendToGrabCycle" => Self::IsCrouchedEnoughForBlendToGrabCycle,
            "TrucksOrDeckInContact" => Self::TrucksOrDeckInContact,
            "PhysicsWantsManualExit" => Self::PhysicsWantsManualExit,
            "ApexReached" => Self::ApexReached,
            _ => unreachable!(),
        }
    }
    pub fn evaluate(&self, host: &MotionHost) -> Result<bool, String> {
        let p = host.gameplay_conditions.as_ref().ok_or("stock gameplay condition requires physical publication")?;
        Ok(match self {
            Self::CanLandOnBoard => p.can_land_on_board,
            //82BA8180: no Air437/time-valid or moving-object gate.
            Self::CanBipedLand => p.offboard_landing_normal[1] > 0.85,
            //82BA5ED8 reads byte312 independently of held311.
            Self::IsDeckFree => host.toggle_board_physical.ok_or("IsDeckFree requires completed OffBoard312")?.free_board,
            //82BA80FC reads completed OffBoard329.
            Self::IsBipedCommittedToMotion => p.offboard_committed_to_motion,
            //82BA42E0 reads Collision3472 OR3475, excluding wheel contacts.
            Self::TrucksOrDeckInContact => p.trucks_or_deck_contact,
            Self::ApexReached => p.reached_apex,
            //82BA8238 compares completed OffBoard116.
            Self::DistToEdge(numeric) => numeric.matches(p.offboard_edge_distance),
            Self::EnoughDistToObstacle { database, animation } => {
                enough_distance(p.offboard_obstacle_distance, host.animation.stock_clip_translation_z(database, animation)?)
            }
            //82BBDE40 compares PhysOutAnimation72 with literal821EE79C.
            Self::IsCrouchedEnoughForBlendToGrabCycle => host.crouching_physical
                .ok_or("Crouch condition requires physical animation height")?.animation_height_72 < f32::from_bits(0x3f19_999a),
            //82BA7930 reads completed Animation168.
            Self::PhysicsWantsManualExit => host.manual_exit.ok_or("PhysicsWantsManualExit requires completed physical animation output")?,
        })
    }
}

fn enough_distance(distance: f32, translation: f32) -> bool {
    //82BA7FF0 fadds;7FF4 fcmpu;7FF8 blt. Preserve unordered behavior.
    !(distance < translation + f32::from_bits(0x3e99_999a))
}

#[cfg(any())]
#[path = "motion_stock_conditions/tests.rs"]
mod tests;
