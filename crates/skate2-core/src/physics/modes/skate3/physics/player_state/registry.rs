//! Coordinator-owned physical state registry.
//!
//! TU3 has one lifecycle object per PhysicalPlayer state. Keeping the
//! support table here makes selection, transition, and diagnostics agree on
//! which owners are actually connected. Task-B-owned states remain explicit
//! unsupported entries until their native adapters are integrated.

use skate3_core::player::state::PhysicalStateId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StateCapability {
    pub id: PhysicalStateId,
    pub supported: bool,
    pub has_enter: bool,
    pub has_exit: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct StateRegistry;

impl StateRegistry {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) fn capability(self, id: PhysicalStateId) -> StateCapability {
        let supported = id.is_grind()
            || id == PhysicalStateId::Nonspecific
            || matches!(
                id,
                PhysicalStateId::Sleeping
                    | PhysicalStateId::PhysicsGround
                    | PhysicalStateId::PhysicsAir
                    | PhysicalStateId::PhysicsAirSecondary
                    | PhysicalStateId::FootPlant | PhysicalStateId::Boneless | PhysicalStateId::HandPlant | PhysicalStateId::RevertGround
                    | PhysicalStateId::KnownAir
                    | PhysicalStateId::BipedAir
                    | PhysicalStateId::BipedGround
                    | PhysicalStateId::OffBoardPushing
                    | PhysicalStateId::GroundAnimation
                    | PhysicalStateId::SlideGround
                    | PhysicalStateId::WipeoutGround
                    | PhysicalStateId::Teleporting
                    | PhysicalStateId::LandingOnDeck
            );
        StateCapability {
            id,
            supported,
            has_enter: supported && !matches!(id, PhysicalStateId::Sleeping),
            has_exit: supported,
        }
    }

    pub(crate) fn can_transition(
        self,
        current: PhysicalStateId,
        requested: PhysicalStateId,
    ) -> bool {
        if !self.capability(current).supported || !self.capability(requested).supported {
            return false;
        }
        if current != PhysicalStateId::Sleeping
            && (current.is_grind()
                || requested.is_grind()
                || current == PhysicalStateId::Nonspecific
                || requested == PhysicalStateId::Nonspecific)
        {
            return true;
        }
        matches!(
            (current, requested),
            (PhysicalStateId::Sleeping, PhysicalStateId::PhysicsGround)
                | (
                    PhysicalStateId::PhysicsGround
                        | PhysicalStateId::PhysicsAir
                    | PhysicalStateId::PhysicsAirSecondary
                        | PhysicalStateId::FootPlant | PhysicalStateId::Boneless | PhysicalStateId::HandPlant | PhysicalStateId::RevertGround
                    | PhysicalStateId::KnownAir
                        | PhysicalStateId::BipedAir
                        | PhysicalStateId::BipedGround
                        | PhysicalStateId::OffBoardPushing
                        | PhysicalStateId::GroundAnimation
                        | PhysicalStateId::SlideGround
                        | PhysicalStateId::WipeoutGround
                        | PhysicalStateId::Teleporting
                        | PhysicalStateId::LandingOnDeck,
                    PhysicalStateId::PhysicsGround
                        | PhysicalStateId::PhysicsAir
                    | PhysicalStateId::PhysicsAirSecondary
                        | PhysicalStateId::FootPlant | PhysicalStateId::Boneless | PhysicalStateId::HandPlant | PhysicalStateId::RevertGround
                    | PhysicalStateId::KnownAir
                        | PhysicalStateId::BipedAir
                        | PhysicalStateId::BipedGround
                        | PhysicalStateId::OffBoardPushing
                        | PhysicalStateId::GroundAnimation
                        | PhysicalStateId::SlideGround
                        | PhysicalStateId::WipeoutGround
                        | PhysicalStateId::Teleporting
                        | PhysicalStateId::LandingOnDeck
                )
        )
    }
}

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    fn grind_trick_darkslide_exit_and_return_have_connected_lifecycles() {
        use PhysicalStateId::*;
        let registry = StateRegistry::new();
        assert!(registry.can_transition(GrindDarkslide, PhysicsAirSecondary));
        assert!(registry.can_transition(PhysicsAirSecondary, PhysicsAir));
        assert!(registry.can_transition(PhysicsAirSecondary, WipeoutGround));
        assert!(registry.can_transition(PhysicsAirSecondary, Teleporting));
        assert!(!registry.can_transition(Sleeping, PhysicsAirSecondary));
        let state = registry.capability(PhysicsAirSecondary);
        assert!(state.has_enter && state.has_exit);
    }

    #[test]
    fn registry_exposes_connected_biped_state_lifecycles() {
        let registry = StateRegistry::new();
        assert!(
            registry
                .capability(PhysicalStateId::PhysicsGround)
                .supported
        );
        assert!(
            registry.can_transition(PhysicalStateId::PhysicsGround, PhysicalStateId::PhysicsAir)
        );
        assert!(!registry.can_transition(PhysicalStateId::Sleeping, PhysicalStateId::PhysicsAir));
        assert!(registry.capability(PhysicalStateId::BipedAir).supported);
    }
}
