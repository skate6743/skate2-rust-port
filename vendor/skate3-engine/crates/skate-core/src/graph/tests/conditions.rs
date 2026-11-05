use super::*;

fn numeric(comparison: Comparison, threshold: f32) -> NumericCondition {
    NumericCondition {
        comparison,
        threshold,
        absolute: false,
    }
}

#[test]
fn comparison_preserves_presence_and_unordered_branch_semantics() {
    let none = numeric(Comparison::None, 0.0);
    let condition = ActionCondition::HasIntent {
        name: "Pushing".into(),
        numeric: none,
    };
    let mut intents = IntentMap::new();
    let input = ConditionInputs::default();
    assert!(!condition.evaluate(&input, &intents, None, &[]).unwrap());
    intents.insert("Pushing", 0.0);
    assert!(condition.evaluate(&input, &intents, None, &[]).unwrap());
    assert!(!none.matches(0.0));
    assert!(numeric(Comparison::GreaterAbsolute, 0.5).matches(-0.75));
    assert!(numeric(Comparison::GreaterEqual, 1.0).matches(f32::NAN));
    assert!(numeric(Comparison::LessEqual, 1.0).matches(f32::NAN));
    assert!(!numeric(Comparison::Greater, 1.0).matches(f32::NAN));
    assert!(!numeric(Comparison::Less, 1.0).matches(f32::NAN));
}

#[test]
fn speed_and_state_conditions_require_their_real_distinct_outputs() {
    let mut input = ConditionInputs::default();
    let speed = ActionCondition::Speed {
        along_skate_z: true,
        numeric: numeric(Comparison::Less, 0.0),
    };
    assert!(
        speed
            .evaluate(&input, &IntentMap::new(), None, &[])
            .is_err()
    );
    input.speeds = Some(SpeedInputs {
        speed: 4.0,
        forward_speed: -3.0,
        speed_and_slope: 2.0,
    });
    assert!(
        speed
            .evaluate(&input, &IntentMap::new(), None, &[])
            .unwrap()
    );
    let state = ActionCondition::CurrentState {
        name: "Riding".into(),
        target: Some(1),
    };
    let parents = [None, Some(0), Some(1), Some(0)];
    assert!(
        state
            .evaluate(&input, &IntentMap::new(), Some(1), &parents)
            .unwrap()
    );
    assert!(
        state
            .evaluate(&input, &IntentMap::new(), Some(2), &parents)
            .unwrap()
    );
    assert!(
        !state
            .evaluate(&input, &IntentMap::new(), Some(3), &parents)
            .unwrap()
    );
    assert!(
        !state
            .evaluate(&input, &IntentMap::new(), None, &parents)
            .unwrap()
    );
}

#[test]
fn push_brake_uses_ground_angle_and_skeleton_override() {
    let ground = PushBrakeInputs {
        ground_axis_y: 1.0,
        skeleton_disables_push_brake: false,
        maximum_ground_angle_degrees: 45.0,
    };
    assert!(!ground.disabled());
    assert!(
        PushBrakeInputs {
            ground_axis_y: 0.0,
            ..ground
        }
        .disabled()
    );
    assert!(
        PushBrakeInputs {
            skeleton_disables_push_brake: true,
            ..ground
        }
        .disabled()
    );
}
