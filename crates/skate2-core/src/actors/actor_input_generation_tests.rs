use super::*;
use crate::{
    action_graph_input_listener::fill::{ActionInputBoolCvar, ActionInputFloatCvar},
    actor_graph_construction::ActorGraphIntents,
    reckoning_numeric::HostReckoningNumeric,
};

pub(super) struct Unused;
impl ActionInputFillCvars for Unused {
    type Error = ();
    fn capture_float(&mut self, _: ActionInputFloatCvar) -> Result<(), ()> {
        panic!("force-brake suppresses Fill")
    }
    fn capture_boolean(&mut self, _: ActionInputBoolCvar) -> Result<(), ()> {
        panic!("force-brake suppresses Fill")
    }
    fn float(&mut self, _: ActionInputFloatCvar) -> Result<f32, ()> {
        panic!("force-brake suppresses Fill")
    }
    fn boolean(&mut self, _: ActionInputBoolCvar) -> Result<bool, ()> {
        panic!("force-brake suppresses Fill")
    }
}
impl ActionInputJuice for Unused {
    type Error = ();
    fn is_trick_active(&mut self) -> Result<bool, ()> {
        panic!("force-brake suppresses Juice")
    }
    fn take_trick_name(&mut self) -> Result<String, ()> {
        panic!("force-brake suppresses Juice")
    }
}
struct Physics {
    result: Result<bool, &'static str>,
    calls: usize,
}
impl ActorInputPhysics for Physics {
    type Error = &'static str;
    fn physics_state(&mut self) -> Result<u32, Self::Error> {
        panic!("force-brake suppresses Fill PhysOut query")
    }
    fn effective_anim_z(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        panic!("force-brake suppresses Fill PhysOut query")
    }
    fn is_wiping_out(&mut self) -> Result<bool, Self::Error> {
        self.calls += 1;
        self.result
    }
}
#[test]
fn forced_brake_clears_map_skips_fill_and_applies_pending_wipeout_before_consuming_flag() {
    for outcome in [Ok(false), Ok(true), Err("physout")] {
        let (actor, _route) = super::super::tests::actor_with_listener();
        actor.try_construct_graphs().unwrap();
        {
            let mut state = actor.prefix().unwrap();
            state.input_generation_flags = Some(0x8400_0000);
            let map = state
                .graphs
                .as_mut()
                .unwrap()
                .action_mut()
                .unwrap()
                .intents
                .as_mut()
                .unwrap();
            map.insert_unique(ActorGraphIntents::native_name(b"OldIntent"), 2.0)
                .unwrap();
        }
        let mut physics = Physics {
            result: outcome,
            calls: 0,
        };
        let result = actor.try_generate_action_intents(
            0.25,
            None,
            &mut Unused,
            &mut HostReckoningNumeric,
            &mut Unused,
            &mut physics,
        );
        let state = actor.prefix().unwrap();
        let map = state
            .graphs()
            .unwrap()
            .action()
            .unwrap()
            .intents
            .as_ref()
            .unwrap();
        assert!(
            map.get(&ActorGraphIntents::native_name(b"OldIntent"))
                .is_none()
        );
        assert_eq!(physics.calls, 1);
        if outcome.is_err() {
            assert!(matches!(
                result,
                Err(ActorInputGenerationError::Physics("physout"))
            ));
            assert_eq!(map.len(), 0);
            assert_eq!(state.input_generation_flags, Some(0x8400_0000));
        } else {
            result.unwrap();
            assert_eq!(state.input_generation_flags, Some(0x0400_0000));
            assert_eq!(
                map.get(&ActorGraphIntents::native_name(b"ForceBrake"))
                    .map(f32::to_bits),
                Some(1.0_f32.to_bits())
            );
            assert_eq!(
                map.get(&ActorGraphIntents::native_name(b"WipeOutRequest"))
                    .is_some(),
                outcome == Ok(false)
            );
        }
    }
}
