//! Stock condition attributes bound to core evaluations. Values use the binary
//! float/bool encodings directly, including constructor precedence and defaults.
use skate3_core::graph::conditions::{ActionCondition, Comparison, NumericCondition};
use skate_data::state_graph::attributes::Attributes;

///Original82BA3A78: first cached record, sequence filter, then numeric scalar
///comparison only for kinds0/2. Shared by the ActionGraph and MotionGraph.
pub(crate) fn animation_attribute(
    attributes: &[skate3_core::animation::output::attributes::AnimationAttribute],
    name: skate3_core::animation::output::attributes::AttributeName,
    sequence_id: i32,
    numeric: NumericCondition,
) -> bool {
    attributes.iter().find(|a| a.name == name).is_some_and(|a| {
        (sequence_id == -1 || sequence_id == a.sequence_id)
            && (numeric.comparison == Comparison::None
                || !matches!(a.kind, 0 | 2)
                || a.payload.0[0].is_some_and(|word| numeric.matches(f32::from_bits(word))))
    })
}
///Original82BA45D0/82BA4658 rejects an inactive containing state's time.
pub(crate) fn state_time(
    frame: &skate3_core::graph::controller::Frame,
    target: Option<usize>,
    numeric: NumericCondition,
) -> bool {
    target
        .and_then(|id| frame.state_times.get(id).copied().flatten())
        .is_some_and(|time| !(time < 0.0) && numeric.matches(time))
}

pub(super) fn parse(attributes: &Attributes<'_>) -> Result<Option<ActionCondition>, String> {
    use ActionCondition as C;
    Ok(Some(match attributes.text("name").unwrap_or("") {
        "HasAGIntent" => C::HasIntent {
            name: attributes.text("intent").unwrap_or("").to_owned(),
            numeric: numeric(attributes),
        },
        "TimeSinceLastInput" => C::TimeSinceLastInput(numeric(attributes)),
        "PhysicsSpeedCompare" => C::Speed {
            along_skate_z: attributes.boolean_byte("alongSkateZ", 0) != 0,
            numeric: numeric(attributes),
        },
        "PhysicsSpeedAndSlopeCompare" => C::SpeedAndSlope(numeric(attributes)),
        "PhysFilteredState" => C::FilteredState(match attributes.text("state").unwrap_or("") {
            "invalid" => 0,
            "ground" => 1,
            "air" => 2,
            "grind" => 3,
            "wipeout" => 4,
            "teleport" => 5,
            "offboard" => 6,
            "offboardair" => 7,
            other => return Err(format!("PhysFilteredState has undefined state `{other}`")),
        }),
        "IsGrinding" => C::Grinding {
            name: attributes.text("grindName").map(str::to_owned),
        },
        "IsMirrored" => C::Mirrored,
        "IsRidingFakie" => C::RidingFakie,
        "DisablePushBrake" => C::DisablePushBrake,
        "CurrentState" => C::CurrentState {
            name: attributes.text("state").unwrap_or("").to_owned(),
            target: None,
        },
        _ => return Ok(None),
    }))
}

pub(crate) fn numeric(attributes: &Attributes<'_>) -> NumericCondition {
    use Comparison as C;
    // TU382C126F0. The long spelling takes precedence over all short forms;
    // unrecognized long spellings retain the constructor's Equals default.
    //82AE89B0 confirms case-sensitive comparison.
    let (comparison, field) = if let Some(value) = attributes.text("comparison") {
        (
            match value {
                "NotEqual" => C::NotEqual,
                "GreaterThan" => C::Greater,
                "LessThan" => C::Less,
                "GreaterEqual" => C::GreaterEqual,
                "LessEqual" => C::LessEqual,
                _ => C::Equal,
            },
            "value",
        )
    } else {
        [
            (C::Equal, "equal"),
            (C::NotEqual, "notEqual"),
            (C::Greater, "greater"),
            (C::GreaterAbsolute, "greaterThanAbs"),
            (C::Less, "less"),
            (C::GreaterEqual, "greaterEqual"),
            (C::LessEqual, "lessEqual"),
        ]
        .into_iter()
        .find(|(_, field)| attributes.get(field).is_some())
        .unwrap_or((C::None, ""))
    };
    NumericCondition {
        comparison,
        threshold: f32::from_bits(attributes.float_bits(field, 0)),
        absolute: attributes.boolean_byte("abs", 0) != 0,
    }
}

pub(super) fn bind_current_states(
    graph: &crate::skate3_native::graph_runtime::LoadedGraph,
    instances: &mut super::action_nodes::ActionInstances,
) {
    let mut element_parents = vec![None; graph.source.elements.len()];
    for (parent, element) in graph.source.elements.iter().enumerate() {
        for &child in &element.children {
            element_parents[child] = Some(parent);
        }
    }
    for &operation in &graph.runtime.operations.conditions {
        if let super::action_nodes::ActionOperation::ExtraCondition(
            super::action_conditions::ActionCondition::ParentTime { target, .. },
        ) = &mut instances.operations[operation].operation
        {
            let mut element = element_parents[graph.binding.operations[operation].element];
            while let Some(id) = element {
                if let Some(state) = graph.binding.states.iter().position(|s| s.element == id) {
                    *target = Some(state);
                    break;
                }
                element = element_parents[id];
            }
            continue;
        }
        let super::action_nodes::ActionOperation::Condition(ActionCondition::CurrentState {
            name,
            target,
        }) = &mut instances.operations[operation].operation
        else {
            continue;
        };
        // Node::GetParentState82C11B88 climbs through expressions/transitions
        // to the containing state; it is not the graph's current leaf.
        let mut element = element_parents[graph.binding.operations[operation].element];
        while let Some(id) = element {
            if let Some(state) = graph.binding.states.iter().position(|s| s.element == id) {
                *target = graph.binding.find_state(state, name, true); //82C13728
                break;
            }
            element = element_parents[id];
        }
    }
}
