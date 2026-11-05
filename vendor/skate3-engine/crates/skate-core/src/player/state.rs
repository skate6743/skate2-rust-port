//! Physical-player state identities and native object ownership from
//! `PhysicalPlayer` construction and `SetPhysicsState` (`0x82DB8540`).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PhysicalStateId {
    PhysicsGround = 100,
    SlideGround = 101,
    RevertGround = 102,
    GroundAnimation = 103,
    Skitching = 104,
    FollowPath = 105,
    PhysicsAir = 200,
    KnownAir = 201,
    PhysicsAirSecondary = 202,
    WipeoutGround = 300,
    GrindBoardslide = 400,
    GrindFiftyFifty = 401,
    GrindTipslide = 402,
    GrindFiveO = 403,
    GrindBackslash = 404,
    GrindDarkslide = 405,
    BipedGround = 500,
    BipedAir = 501,
    OffBoardPushing = 502,
    LandingOnDeck = 503,
    HandPlant = 600,
    FootPlant = 601,
    Boneless = 602,
    Sleeping = 700,
    Nonspecific = 701,
    Teleporting = 702,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownPhysicalState(pub u32);

impl TryFrom<u32> for PhysicalStateId {
    type Error = UnknownPhysicalState;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let state = match value {
            100 => Self::PhysicsGround,
            101 => Self::SlideGround,
            102 => Self::RevertGround,
            103 => Self::GroundAnimation,
            104 => Self::Skitching,
            105 => Self::FollowPath,
            200 => Self::PhysicsAir,
            201 => Self::KnownAir,
            202 => Self::PhysicsAirSecondary,
            300 => Self::WipeoutGround,
            400 => Self::GrindBoardslide,
            401 => Self::GrindFiftyFifty,
            402 => Self::GrindTipslide,
            403 => Self::GrindFiveO,
            404 => Self::GrindBackslash,
            405 => Self::GrindDarkslide,
            500 => Self::BipedGround,
            501 => Self::BipedAir,
            502 => Self::OffBoardPushing,
            503 => Self::LandingOnDeck,
            600 => Self::HandPlant,
            601 => Self::FootPlant,
            602 => Self::Boneless,
            700 => Self::Sleeping,
            701 => Self::Nonspecific,
            702 => Self::Teleporting,
            other => return Err(UnknownPhysicalState(other)),
        };
        Ok(state)
    }
}

impl PhysicalStateId {
    pub const ALL: [Self; 26] = [
        Self::PhysicsGround,
        Self::SlideGround,
        Self::RevertGround,
        Self::GroundAnimation,
        Self::Skitching,
        Self::FollowPath,
        Self::PhysicsAir,
        Self::KnownAir,
        Self::PhysicsAirSecondary,
        Self::WipeoutGround,
        Self::GrindBoardslide,
        Self::GrindFiftyFifty,
        Self::GrindTipslide,
        Self::GrindFiveO,
        Self::GrindBackslash,
        Self::GrindDarkslide,
        Self::BipedGround,
        Self::BipedAir,
        Self::OffBoardPushing,
        Self::LandingOnDeck,
        Self::HandPlant,
        Self::FootPlant,
        Self::Boneless,
        Self::Sleeping,
        Self::Nonspecific,
        Self::Teleporting,
    ];

    /// Offset of the owning state pointer inside `PhysicalPlayer`.
    pub const fn native_owner_offset(self) -> u32 {
        match self {
            Self::Sleeping => 1692,
            Self::PhysicsGround => 1696,
            Self::SlideGround => 1700,
            Self::RevertGround => 1704,
            Self::GroundAnimation => 1708,
            Self::PhysicsAir => 1712,
            Self::PhysicsAirSecondary => 1716,
            Self::KnownAir => 1720,
            Self::GrindDarkslide => 1724,
            Self::GrindBoardslide => 1728,
            Self::GrindFiftyFifty => 1732,
            Self::GrindTipslide => 1736,
            Self::GrindFiveO => 1740,
            Self::GrindBackslash => 1744,
            Self::WipeoutGround => 1748,
            Self::Nonspecific => 1752,
            Self::Teleporting => 1756,
            Self::FollowPath => 1760,
            Self::BipedGround => 1764,
            Self::BipedAir => 1768,
            Self::Skitching => 1772,
            Self::OffBoardPushing => 1776,
            Self::HandPlant => 1780,
            Self::FootPlant => 1784,
            Self::Boneless => 1788,
            Self::LandingOnDeck => 1792,
        }
    }

    /// Category written by native `GetStateCategory` into ProcessedPhysIn.
    pub const fn category(self) -> u32 {
        (self as u32 / 100) * 100
    }

    pub const fn is_grind(self) -> bool {
        matches!(
            self,
            Self::GrindBoardslide
                | Self::GrindFiftyFifty
                | Self::GrindTipslide
                | Self::GrindFiveO
                | Self::GrindBackslash
                | Self::GrindDarkslide
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_native_state_round_trips_and_has_a_unique_owner_slot() {
        let mut slots = PhysicalStateId::ALL.map(PhysicalStateId::native_owner_offset);
        slots.sort_unstable();
        assert!(slots.windows(2).all(|pair| pair[0] != pair[1]));
        for state in PhysicalStateId::ALL {
            assert_eq!(PhysicalStateId::try_from(state as u32), Ok(state));
            assert_eq!(state.category(), (state as u32 / 100) * 100);
        }
    }

    #[test]
    fn only_the_six_native_grind_states_form_the_grind_range() {
        for state in PhysicalStateId::ALL {
            assert_eq!(state.is_grind(), (400..=405).contains(&(state as u32)));
        }
    }

    #[test]
    fn unknown_state_ids_are_rejected_without_a_fallback() {
        assert_eq!(PhysicalStateId::try_from(0), Err(UnknownPhysicalState(0)));
        assert_eq!(
            PhysicalStateId::try_from(203),
            Err(UnknownPhysicalState(203))
        );
    }
}
