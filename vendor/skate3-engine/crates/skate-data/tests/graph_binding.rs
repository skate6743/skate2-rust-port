use skate_data::state_graph::{
    GraphAttribute, GraphElement, StateGraph,
    binding::{Binding, parse_condition_mask},
};

fn attr(name: &str, text: &str, byte: u8) -> GraphAttribute {
    GraphAttribute {
        name: name.into(),
        text: text.into(),
        float_bits: 0,
        boolean_byte: byte,
    }
}
fn element(tag: &str, attributes: Vec<GraphAttribute>, children: Vec<usize>) -> GraphElement {
    GraphElement {
        source_offset: 0,
        tag: tag.into(),
        attributes,
        children,
    }
}
#[test]
fn constructor_flags_priorities_expressions_and_target_resolution() {
    let source = StateGraph {
        elements: vec![
            element("state", vec![attr("name", "Root", 0)], vec![1, 3, 4, 6]),
            element(
                "state",
                vec![attr("name", "Off", 0), attr("enabled", "true", 0)],
                vec![2],
            ),
            element(
                "behaviour",
                vec![attr("name", "Handler", 0), attr("enabled", "true", 255)],
                vec![],
            ),
            element(
                "transition",
                vec![attr("target", "Off", 0), attr("priority", "urgent", 0)],
                vec![],
            ),
            element(
                "state",
                vec![attr("name", "On", 0), attr("interruptable", "Root", 0)],
                vec![5],
            ),
            element(
                "transition",
                vec![attr("target", "Root.On", 0), attr("priority", "med", 0)],
                vec![],
            ),
            element("transition", vec![attr("target", "Absent", 0)], vec![7]),
            element("expression", vec![attr("op", "or", 0)], vec![8]),
            element("condition", vec![attr("name", "Condition", 0)], vec![]),
        ],
    };
    let binding = Binding::from_graph(&source).unwrap();
    assert_eq!(binding.states[0].children, vec![1, 2]);
    assert_eq!(binding.states[1].enabled, 0);
    assert_eq!(binding.operations[0].enabled, 0); // Inherited disable wins.
    assert_eq!(binding.states[2].interruptibility, 2);
    assert_eq!(binding.states[2].interrupt_ancestor, Some(0));
    assert_eq!(binding.transitions[0].target, Some(1));
    assert_eq!(binding.transitions[0].enabled, 0); // Target disables transition.
    assert_eq!(binding.transitions[0].priority, 3);
    assert_eq!(binding.transitions[1].target, Some(2));
    assert_eq!(binding.transitions[1].priority, 1);
    assert_eq!(binding.transitions[2].target, None);
    assert_eq!(binding.transitions[2].enabled, 0);
    assert_eq!(binding.expressions[0].operator, 2);
    assert_eq!(binding.operations[1].condition_mask, Some(1));
    // Constructor inheritance precedes Process disabling an unresolved target.
    assert_eq!(binding.expressions[0].enabled, 1);
}

#[test]
fn condition_masks_match_tu3_parse_mask_ordinals() {
    assert_eq!(parse_condition_mask(None), 1);
    assert_eq!(parse_condition_mask(Some("precond")), 1);
    assert_eq!(parse_condition_mask(Some("sustain")), 2);
    assert_eq!(parse_condition_mask(Some("always")), 3);
    assert_eq!(parse_condition_mask(Some("postcond")), 4);
    assert_eq!(parse_condition_mask(Some("PRECOND")), 1);
    assert_eq!(parse_condition_mask(Some("unknown")), 1);
}

#[test]
fn missing_native_operations_and_invalid_ancestry_are_explicit() {
    let graph = StateGraph {
        elements: vec![element("include", vec![], vec![])],
    };
    assert!(
        Binding::from_graph(&graph)
            .unwrap_err()
            .0
            .contains("unsupported graph element")
    );
    let graph = StateGraph {
        elements: vec![element(
            "state",
            vec![attr("interruptable", "Missing", 0)],
            vec![],
        )],
    };
    assert!(
        Binding::from_graph(&graph)
            .unwrap_err()
            .0
            .contains("unresolved interrupt ancestor")
    );
}
