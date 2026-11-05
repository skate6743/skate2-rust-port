use super::attributes;
use crate::skate3_native::graph_host::{
    motion_conditions::MotionCondition,
    motion_nodes::{MotionFactory, MotionOperation},
};
use skate_data::state_graph::{
    attributes::Attributes,
    binding::{Node, OperationFactory, OperationKind},
};

#[test]
fn production_factory_routes_three_handlers_and_all_five_conditions_to_grind_owner() {
    let mut factory = MotionFactory;
    for name in [
        "CreateGrindAttributes",
        "ControlGrindCrouch",
        "GrindControlFade",
    ] {
        let a = attributes(&[
            ("name", name),
            ("distBoardToCogAnimAttribute", "height"),
            ("twistAnimAttribute", "twist"),
            ("twistMGIntent", "GrindBalanceX"),
        ]);
        assert!(matches!(
            factory
                .create(
                    OperationKind::Behavior,
                    Node::State(0),
                    &Attributes::new(&a)
                )
                .unwrap(),
            Some(MotionOperation::Grind(_))
        ));
        assert!(!crate::skate3_native::graph_host::motion_stock_gameplay::Operation::recognizes(name));
    }
    for name in [
        "IsGrindBluntingBackslash",
        "IsGrindApproach",
        "GrindTrickOutTypeAllowed",
        "IsLandingIntoGrind",
        "IsDroppingIn",
    ] {
        let a = attributes(&[("name", name), ("type", "right"), ("Facing", "B")]);
        assert!(matches!(
            factory
                .create(
                    OperationKind::Condition,
                    Node::State(0),
                    &Attributes::new(&a)
                )
                .unwrap(),
            Some(MotionOperation::Condition(MotionCondition::Grind(_)))
        ));
        assert!(!crate::skate3_native::graph_host::motion_stock_conditions::Condition::recognizes(name));
    }
}

#[test]
fn production_factory_preserves_required_grind_attribute_errors() {
    let mut factory = MotionFactory;
    for (name, kind) in [
        ("GrindControlFade", OperationKind::Behavior),
        ("GrindTrickOutTypeAllowed", OperationKind::Condition),
    ] {
        let a = attributes(&[("name", name)]);
        assert!(
            factory
                .create(kind, Node::State(0), &Attributes::new(&a))
                .is_err()
        );
    }
}
