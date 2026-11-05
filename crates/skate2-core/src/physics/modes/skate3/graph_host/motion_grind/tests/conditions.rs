use super::super::conditions::{Condition, Physical};
use super::attributes;
use skate_data::state_graph::attributes::Attributes;
fn physical() -> Physical {
    Physical {
        filtered_grinding_80: false,
        blunting_136: 4,
        approach_268: 1,
        trick_out_240: 2,
        air_grind_443: true,
        air_time_184: 0.125,
        dropping_in_324: true,
    }
}
#[test]
fn conditions_use_their_actual_fields_not_a_shared_grinding_proxy() {
    let mut p = physical();
    assert!(Condition::BluntingBackslash.evaluate(&p));
    assert!(Condition::TrickOutType { value: 2 }.evaluate(&p));
    assert!(Condition::LandingIntoGrind.evaluate(&p));
    assert!(Condition::DroppingIn.evaluate(&p));
    assert!(!Condition::Approach { forwards: true }.evaluate(&p));
    p.filtered_grinding_80 = true;
    assert!(Condition::Approach { forwards: true }.evaluate(&p));
    assert!(!Condition::Approach { forwards: false }.evaluate(&p));
    p.approach_268 = 0;
    assert!(Condition::Approach { forwards: false }.evaluate(&p));
    p.air_time_184 = 0.2;
    assert!(!Condition::LandingIntoGrind.evaluate(&p));
    p.air_time_184 = 0.0;
    p.air_grind_443 = false;
    assert!(!Condition::LandingIntoGrind.evaluate(&p));
}
#[test]
fn approach_case_folds_facing_but_trick_out_type_uses_exact_strings() {
    let a = attributes(&[("name", "IsGrindApproach"), ("Facing", "f")]);
    assert_eq!(
        Condition::parse(&Attributes::new(&a)).unwrap(),
        Some(Condition::Approach { forwards: true })
    );
    for (name, value) in [("normal", 0), ("left", 1), ("right", 2)] {
        let a = attributes(&[("name", "GrindTrickOutTypeAllowed"), ("type", name)]);
        assert_eq!(
            Condition::parse(&Attributes::new(&a)).unwrap(),
            Some(Condition::TrickOutType { value })
        );
    }
    let a = attributes(&[("name", "GrindTrickOutTypeAllowed"), ("type", "Left")]);
    assert!(Condition::parse(&Attributes::new(&a)).is_err());
}
