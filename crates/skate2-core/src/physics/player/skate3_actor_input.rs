//! Semantic S2 Actor policy at the native S3 input boundary.
//! S2 GenerateActionGraphIntents825FDA28 and Actor.Update825F10F0 own the
//! requests; S3 input-filter bit numbers are deliberately not reused.
use super::super::PlayerControls;
const GESTURES: [&str; 8] = [
    "GestureDownStart",
    "GestureLeftStart",
    "GestureRightStart",
    "GestureUpStart",
    "GestureDownHeld",
    "GestureLeftHeld",
    "GestureRightHeld",
    "GestureUpHeld",
];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HostActorInput {
    pub pinned: bool,
    pub pinned_gestures: bool,
    pub prevent_manual_respawn: bool,
    request_wipeout: bool,
}
impl HostActorInput {
    pub(crate) fn from_skate2(flags: u32, pinned_gestures: bool) -> Self {
        Self {
            pinned: flags & 0x0400_0000 != 0,
            pinned_gestures,
            prevent_manual_respawn: flags & 0x1000_0000 != 0,
            request_wipeout: flags & 0x8000_0000 != 0,
        }
    }
    pub(crate) fn apply_map(
        self,
        map: &mut skate3_core::graph::intents::IntentMap,
        wiping_out: bool,
    ) {
        if self.pinned {
            let gestures: Vec<_> = GESTURES
                .iter()
                .filter_map(|&name| map.get(name).map(|&value| (name, value)))
                .collect();
            map.clear();
            if self.pinned_gestures {
                for (name, value) in gestures {
                    map.insert(name, value);
                }
            }
        }
        for (name, active) in [
            ("WipeOutRequest", self.request_wipeout && !wiping_out),
            ("ForceBrake", self.pinned),
        ] {
            if active {
                map.insert(name, 1.);
            }
        }
    }
    pub(crate) fn apply(self, controls: &mut PlayerControls, wiping_out: bool) {
        self.apply_map(&mut controls.action_intents, wiping_out);
        if self.pinned {
            let allowed = |name: &str| self.pinned_gestures && GESTURES.contains(&name);
            controls.named_intents.retain(|name, _| allowed(name));
            controls.intents.retain(|intent| allowed(intent.name));
        }
        for (name, active) in [
            ("WipeOutRequest", self.request_wipeout && !wiping_out),
            ("ForceBrake", self.pinned),
        ] {
            if active {
                controls.named_intents.insert(name.into(), 1.);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_actor_keeps_only_its_authorized_gestures_and_forces_braking() {
        for gestures in [false, true] {
            let mut controls = PlayerControls::default();
            for name in [
                "Pushing",
                "OB_Dismount",
                "Trick",
                "Kickflip",
                "GestureUpStart",
                "GestureDownStart",
                "GestureLeftStart",
                "GestureRightStart",
            ] {
                controls.action_intents.insert(name, 1.);
                controls.named_intents.insert(name.into(), 1.);
            }
            HostActorInput::from_skate2(0x0400_0000, gestures).apply(&mut controls, false);
            assert!(controls.action_intents.contains_key("ForceBrake"));
            assert_eq!(controls.action_intents.len(), if gestures { 5 } else { 1 });
            for name in [
                "GestureUpStart",
                "GestureDownStart",
                "GestureLeftStart",
                "GestureRightStart",
            ] {
                assert_eq!(controls.action_intents.contains_key(name), gestures);
            }
        }
    }
    #[test]
    fn pinned_fill_retains_controller_history_until_unpin_or_gesture_exception() {
        use skate3_core::input::{gameplay_map::GameplayActions, offboard_intentions};
        for gestures in [false, true] {
            let mut controls = PlayerControls::default();
            let before = *controls.controller.words();
            controls.host_actor_input = HostActorInput::from_skate2(0x0400_0000, gestures);
            let mut values = [0.; 18];
            values[(79 - 64) as usize] = 1.; // Actual stock GP_YFace.
            let mut actions = GameplayActions::from_values(values);
            controls.update(&mut actions, 1. / 60., 0.5, 0);
            assert_eq!(*controls.controller.words() == before, !gestures);
            controls.host_actor_input = HostActorInput::default();
            controls.update(&mut actions, 1. / 60., 0.5, 0);
            let intents = offboard_intentions::produce_discrete(&controls.controller, 0, false);
            assert_eq!(
                intents
                    .iter()
                    .any(|intent| intent.name == "NewToggleOffBoardState"),
                !gestures
            );
        }
    }
    #[test]
    fn actor_words_map_by_semantics_and_pending_wipeout_is_not_repeated_during_bail() {
        let policy = HostActorInput::from_skate2(0x9000_0000, false);
        assert!(policy.prevent_manual_respawn);
        assert!(!policy.pinned);
        for wiping_out in [false, true] {
            let mut controls = PlayerControls::default();
            controls.action_intents.insert("Pushing", 1.);
            policy.apply(&mut controls, wiping_out);
            assert!(controls.action_intents.contains_key("Pushing"));
            assert_eq!(
                controls.action_intents.contains_key("WipeOutRequest"),
                !wiping_out
            );
            assert!(!controls.action_intents.contains_key("ForceBrake"));
        }
        let unrelated = HostActorInput::from_skate2(0x0800_0040, false);
        assert!(
            !unrelated.pinned && !unrelated.prevent_manual_respawn && !unrelated.request_wipeout
        );
    }
}
