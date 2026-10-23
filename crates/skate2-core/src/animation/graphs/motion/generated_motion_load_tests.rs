use super::*;

#[test]
fn shared_create_address_does_not_conflate_filter_and_enter_skitching_occurrences() {
    let filter = data::BEHAVIOURS[648].name;
    let filter_magnitude = data::BEHAVIOURS[649].name;
    let skitching = data::BEHAVIOURS[175].name;
    assert_eq!(filter, "FilterMotionGraphIntent");
    assert_eq!(filter_magnitude, filter);
    assert_eq!(data::BEHAVIOURS[648].parent, 251);
    assert_eq!(data::BEHAVIOURS[649].parent, 251);
    assert_eq!(skitching, "EnterSkitchingBehaviour");
    let filter_callbacks = construct::binding(filter).unwrap().callbacks;
    let skitching_callbacks = construct::binding(skitching).unwrap().callbacks;
    assert_eq!(filter_callbacks[0], 0x82ca_7628);
    assert_eq!(filter_callbacks[0], skitching_callbacks[0]);
    assert_eq!(filter_callbacks[1], 0x82cd_5e30);
    assert_eq!(skitching_callbacks[1], 0x82ca_7620);
}

#[test]
fn every_nonvault_behaviour_and_condition_accepts_its_original_declaration() {
    let cvars = crate::input_cvar_runtime::InputCvarReferences::new_static_initialized();
    let mut manager = crate::input_cvar_runtime::InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    for (index, declaration) in data::BEHAVIOURS.iter().enumerate() {
        assert!(
            construct::binding(declaration.name).is_some(),
            "behaviour {index}"
        );
        if matches!(
            declaration.name,
            "EnterSkitchingBehaviour"
                | "SkitchingBehaviour"
                | "SkitchShimmyingBehaviour"
                | "MovingObject"
                | "HippyJumpAntic"
        ) {
            continue;
        }
        let mut object = construct::behaviour(declaration, Arc::from(declaration.name))
            .unwrap_or_else(|error| panic!("behaviour {index}: {error}"));
        for parameter in 0..usize::from(declaration.parameters.count) {
            let parameter_index =
                usize::from(data::INDICES[usize::from(declaration.parameters.index) + parameter]);
            construct::add_parameter(&mut object, data::PARAMETERS[parameter_index], index)
                .unwrap();
        }
    }
    for (index, declaration) in data::HOOKS.iter().enumerate() {
        hooks::construct(declaration, Arc::from(declaration.name))
            .unwrap_or_else(|error| panic!("hook {index}: {error}"));
    }
    for (index, declaration) in data::CONDITIONS.iter().enumerate() {
        conditions::construct(declaration, Arc::from(declaration.name), &mut || {
            cvars
                .try_mo_angle_max_delta()
                .map_err(MotionLoadError::Cvar)
        })
        .unwrap_or_else(|error| panic!("condition {index}: {error}"));
    }
}

#[test]
fn actual_eager_prefix_retains_root_constructors_and_stops_at_missing_family() {
    let statics = GeneratedMotionStatics::new(None, None);
    let mut owner = GeneratedMotionLoad::new(&statics);
    let mut allocations = Vec::new();
    let result = owner.try_construct(&mut |site| {
        allocations.push((site.index, site.binding.name));
        Ok(MotionAllocation::Host)
    });
    assert!(matches!(result, Err(MotionLoadError::MissingVault)));
    assert_eq!(allocations.len(), 176);
    let object = statics.behaviour(2).unwrap().unwrap();
    let construct::MotionBehaviourPayload::UpdateRidingFakie {
        high_speed_threshold,
        low_speed_threshold,
        time_slowly_rolling_backwards_threshold,
        time_from_teleport_threshold,
    } = object.payload
    else {
        panic!("real root payload")
    };
    assert_eq!(high_speed_threshold.to_bits(), (-1.0_f32).to_bits());
    assert_eq!(low_speed_threshold.to_bits(), (-0.5_f32).to_bits());
    assert_eq!(
        time_slowly_rolling_backwards_threshold.to_bits(),
        0.2_f32.to_bits()
    );
    assert_eq!(time_from_teleport_threshold.to_bits(), 1.0_f32.to_bits());
    let first = statics.behaviour(4).unwrap().unwrap();
    let second = statics.behaviour(5).unwrap().unwrap();
    assert!(Arc::ptr_eq(&first.base.name, &second.base.name));
    assert!(Arc::ptr_eq(&first.base.name, &first.base.path));
    assert!(statics.behaviour(175).unwrap().is_none());
    assert!(matches!(
        owner.try_construct(&mut |_| panic!("no host replay")),
        Err(MotionLoadError::AlreadyAttempted)
    ));
}

#[test]
fn native_null_factory_result_continues_and_partial_sinit_is_not_repaired() {
    let statics = GeneratedMotionStatics::new(None, None);
    let mut owner = GeneratedMotionLoad::new(&statics);
    let result = owner.try_construct(&mut |site| {
        Ok(if site.index == 2 {
            MotionAllocation::Null
        } else {
            MotionAllocation::Host
        })
    });
    assert!(matches!(result, Err(MotionLoadError::MissingVault)));
    assert!(statics.behaviour(2).unwrap().is_none());
    assert!(statics.behaviour(3).unwrap().is_some());
    let mut second = GeneratedMotionLoad::new(&statics);
    assert!(matches!(
        second.try_construct(&mut |site| {
            assert_ne!(site.domain, ObjectDomain::Behaviour);
            Ok(MotionAllocation::Host)
        }),
        Err(MotionLoadError::MissingCvars)
    ));
}

#[test]
fn generated_motion_descriptors_preserve_packed_sections_and_original_root_order() {
    // The original tables, then the rows the 1.02 off-board Idle appends.
    assert_eq!(data::STATES.len(), 1354 + 6);
    assert_eq!(data::BEHAVIOURS.len(), 2469 + 16);
    assert_eq!(data::CONDITIONS.len(), 406 + 12);
    assert_eq!(data::HOOKS.len(), 18);
    assert_eq!(data::PARAMETERS.len(), 18);
    assert_eq!(data::TRANSITIONS.len(), 1724 + 100);
    assert_eq!(data::INDICES.len(), 1737 + 107);
    assert_eq!(data::STATES[0].name, "Motion");
    assert_eq!(data::STATES[0].children, Section { count: 7, index: 1 });
    assert_eq!(data::STATES[0].behaviours, Section { count: 4, index: 0 });
    assert_eq!(
        data::BEHAVIOURS[2].attributes,
        Section {
            count: 1,
            index: 210
        }
    );
    for declaration in &data::BEHAVIOURS {
        if declaration.attributes.count != 0 {
            assert!(
                usize::from(declaration.attributes.index)
                    + usize::from(declaration.attributes.count)
                    <= data::ATTRIBUTES.len()
            );
        }
        if declaration.parameters.count != 0 {
            assert!(
                usize::from(declaration.parameters.index)
                    + usize::from(declaration.parameters.count)
                    <= data::INDICES.len()
            );
        }
    }
}

#[test]
fn create_attribute_always_preserves_native_update_store_bug_and_skips_set() {
    let index = data::BEHAVIOURS
        .iter()
        .position(|declaration| {
            if declaration.name != "CreateAttribute" {
                return false;
            }
            let start = usize::from(declaration.attributes.index);
            data::ATTRIBUTES[start..start + usize::from(declaration.attributes.count)]
                .iter()
                .any(|attribute| attribute.key == "always" && attribute.value != "0")
        })
        .expect("original nonzero always case");
    let declaration = &data::BEHAVIOURS[index];
    let object = construct::behaviour(declaration, Arc::from(declaration.name)).unwrap();
    let construct::MotionBehaviourPayload::CreateAttribute(value) = object.payload else {
        panic!("attribute")
    };
    assert!(value.begin.active && value.update.active && value.end.active);
    assert_ne!(value.begin.value.to_bits(), 0.0_f32.to_bits());
    assert_eq!(value.begin.value.to_bits(), value.update.value.to_bits());
    assert_eq!(value.end.value.to_bits(), 0.0_f32.to_bits());
    assert!(!value.settable);
}

#[test]
fn all_generated_play_animation_descriptors_and_ordered_parameters_use_shared_typed_parser() {
    let mut count = 0;
    let mut parameter_count = 0;
    for declaration in data::BEHAVIOURS
        .iter()
        .filter(|declaration| declaration.name == "PlayAnimation")
    {
        let mut object = construct::behaviour(declaration, Arc::from(declaration.name)).unwrap();
        for offset in 0..usize::from(declaration.parameters.count) {
            let index =
                usize::from(data::INDICES[usize::from(declaration.parameters.index) + offset]);
            construct::add_parameter(&mut object, data::PARAMETERS[index], count).unwrap();
            parameter_count += 1;
        }
        count += 1;
    }
    assert_eq!(
        count,
        crate::play_animation::PLAY_ANIMATION_CORPUS_OCCURRENCES
    );
    assert_eq!(
        parameter_count,
        crate::play_animation::PLAY_ANIMATION_CORPUS_PARAMS
    );
}

#[test]
fn native_push_brake_condition_uses_the_shared_condition_factory() {
    let declaration = &data::CONDITIONS[18];
    assert_eq!(declaration.name, "DisablePushBrake");
    let object = conditions::construct(declaration, Arc::from(declaration.name), &mut || {
        panic!("plain predicate does not read angle tuning")
    })
    .unwrap();
    assert!(matches!(
        object,
        conditions::MotionCondition::Shared(crate::generated_action_load::ConditionObject {
            class: crate::generated_action_load::ConditionClass::DisablePushBrake,
            payload: crate::generated_action_load::construct::ConditionPayload::Plain,
            ..
        })
    ));
}

#[test]
fn dismount_landing_and_runout_conditions_keep_their_loaded_owners() {
    use crate::generated_action_load::{
        ConditionClass, construct::ConditionPayload as SharedPayload,
    };
    let dismount = &data::CONDITIONS[10];
    let landing = &data::CONDITIONS[11];
    let runout = &data::CONDITIONS[12];
    assert_eq!(dismount.name, "IsDismounting");
    assert_eq!(landing.name, "IsLanding");
    assert_eq!(runout.name, "IsRunningOut");
    let build = |declaration: &ConditionDeclaration| {
        conditions::construct(declaration, Arc::from(declaration.name), &mut || {
            panic!("no angle read")
        })
        .unwrap()
    };
    assert!(matches!(
        build(dismount),
        conditions::MotionCondition::Shared(crate::generated_action_load::ConditionObject {
            class: ConditionClass::IsDismounting,
            payload: SharedPayload::Comparison(_),
            ..
        })
    ));
    assert!(matches!(
        build(landing),
        conditions::MotionCondition::Motion {
            payload: conditions::ConditionPayload::Comparison {
                class: conditions::plain::ComparisonCondition::IsLanding,
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        build(runout),
        conditions::MotionCondition::Motion {
            payload: conditions::ConditionPayload::Comparison {
                class: conditions::plain::ComparisonCondition::IsRunningOut,
                ..
            },
            ..
        }
    ));
}

#[test]
fn push_current_state_conditions_use_shared_named_payloads() {
    for (index, expected) in [(364, "PushFromFakie"), (365, "Switch")] {
        let declaration = &data::CONDITIONS[index];
        assert_eq!(declaration.name, "CurrentState");
        let object = conditions::construct(declaration, Arc::from(declaration.name), &mut || {
            panic!("no angle read")
        })
        .unwrap();
        let conditions::MotionCondition::Shared(object) = object else {
            panic!("shared condition")
        };
        assert_eq!(
            object.class,
            crate::generated_action_load::ConditionClass::CurrentState
        );
        let crate::generated_action_load::construct::ConditionPayload::CurrentState(name) =
            object.payload
        else {
            panic!("named current state")
        };
        assert_eq!(name, expected);
    }
}

#[test]
fn slide_entry_conditions_retain_authored_left_and_default_right() {
    for (index, expected) in [(318, false), (324, true)] {
        let declaration = &data::CONDITIONS[index];
        assert_eq!(declaration.name, "CanEnterSlide");
        let object = conditions::construct(declaration, Arc::from(declaration.name), &mut || {
            panic!("no angle read")
        })
        .unwrap();
        let conditions::MotionCondition::Motion {
            payload: conditions::ConditionPayload::CanEnterSlide { right },
            ..
        } = object
        else {
            panic!("Motion slide payload")
        };
        assert_eq!(right, expected);
    }
}

#[test]
fn ground_grab_admissions_use_original_motion_intents() {
    for (index, name) in [
        (288, "GroundBSGrab"),
        (343, "GroundFSGrab"),
        (287, "GroundDBLGrab"),
        (225, "Coffin"),
    ] {
        let declaration = &data::CONDITIONS[index];
        let object = conditions::construct(declaration, Arc::from(declaration.name), &mut || {
            panic!("no angle read")
        })
        .unwrap();
        let conditions::MotionCondition::Motion {
            payload: conditions::ConditionPayload::HasIntent { comparison, intent },
            ..
        } = object
        else {
            panic!("Motion intent payload")
        };
        assert_eq!(
            intent,
            crate::play_animation::FastString36::encode(name).unwrap()
        );
        assert_eq!(
            comparison.op,
            crate::generated_action_load::construct::ComparisonOp::Nop
        );
    }
}

#[test]
fn wipeout_time_to_land_retains_authored_comparison_and_unordered_behavior() {
    let declaration = &data::CONDITIONS[403];
    assert_eq!(declaration.name, "WipeoutTimeToLand");
    let object = conditions::construct(declaration, Arc::from(declaration.name), &mut || {
        panic!("no angle read")
    })
    .unwrap();
    let conditions::MotionCondition::Motion {
        payload:
            conditions::ConditionPayload::Comparison {
                class: conditions::plain::ComparisonCondition::WipeoutTimeToLand,
                comparison,
            },
        ..
    } = object
    else {
        panic!("Motion comparison payload")
    };
    assert_eq!(
        comparison.op,
        crate::generated_action_load::construct::ComparisonOp::Less
    );
    assert!(!comparison.absolute);
    assert_eq!(comparison.reference.to_bits(), 0.75_f32.to_bits());
    let compare = crate::generated_action_load::conditions::compare;
    assert!(compare(comparison, 0.1));
    assert!(!compare(comparison, 0.75));
    assert!(!compare(comparison, f32::NAN));
    assert!(!compare(comparison, f32::MAX));
}
