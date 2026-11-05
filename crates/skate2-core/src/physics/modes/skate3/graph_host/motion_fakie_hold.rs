//! Trainer gates for the stock automatic fakie-to-switch animation transitions.
//! FakieTurn is player steering and must never be suppressed by this option.
use skate_data::state_graph::binding::{Binding, Node};

pub(super) fn bind(binding: &Binding) -> Vec<bool> {
    let mut gates = vec![false; binding.operations.len()];
    for transition in &binding.transitions {
        let Some(target) = transition.target else { continue; };
        let mut names = Vec::new();
        let mut state = Some(target);
        while let Some(id) = state {
            names.push(binding.states[id].name.as_str());
            state = binding.states[id].parent;
        }
        names.reverse();
        if names != ["Motion", "OnBoard", "OnGround", "RidingIdle", "Riding", "Turning", "Switch"] {
            continue;
        }
        if let Some(expression) = transition.expression {
            mark(binding, expression, &mut gates);
        }
    }
    gates
}
fn mark(binding: &Binding, expression: usize, gates: &mut [bool]) {
    for child in &binding.expressions[expression].children {
        match *child {
            Node::Operation(id) if binding.operations[id].name == "IsRidingFakie" => gates[id] = true,
            Node::Expression(id) => mark(binding, id, gates),
            _ => {}
        }
    }
}
pub(super) fn blocked(enabled: bool, gates: &[bool], operation: usize) -> bool {
    enabled && gates.get(operation).copied().unwrap_or(false)
}

#[cfg(any())]
mod tests {
    use super::*;
    use skate_data::state_graph::{GraphAttribute, GraphElement, StateGraph};
    struct Tree(&'static str, Vec<(&'static str, &'static str)>, Vec<Tree>);
    fn state(name: &'static str, children: Vec<Tree>) -> Tree { Tree("state", vec![("name", name)], children) }
    fn condition() -> Tree { Tree("condition", vec![("name", "IsRidingFakie")], vec![]) }
    fn transition(target: &'static str) -> Tree {
        Tree("transition", vec![("target", target)], vec![Tree("expression", vec![("op", "and")], vec![condition()])])
    }
    fn add(tree: Tree, graph: &mut StateGraph) -> usize {
        let id = graph.elements.len();
        graph.elements.push(GraphElement { source_offset: id, tag: tree.0.into(),
            attributes: tree.1.into_iter().map(|(name, text)| GraphAttribute {
                name: name.into(), text: text.into(), float_bits: 0, boolean_byte: 0,
            }).collect(), children: vec![] });
        let children = tree.2.into_iter().map(|child| add(child, graph)).collect();
        graph.elements[id].children = children;
        id
    }
    #[test]
    fn gates_idle_and_bump_switch_but_preserves_other_fakie_conditions() {
        let tree = state("Motion", vec![state("OnBoard", vec![state("OnGround", vec![
            state("RidingIdle", vec![
                state("Bumped", vec![transition("Riding.Turning.Switch")]),
                state("Riding", vec![state("Turning", vec![
                    state("Idle", vec![transition("Switch")]), state("Switch", vec![]),
                    state("Push", vec![Tree("expression", vec![], vec![condition()])]),
                ])]),
            ]),
            // Same leaf name elsewhere must not be treated as automatic switching.
            state("Other", vec![state("Idle", vec![transition("Switch")]), state("Switch", vec![])]),
        ])])]);
        let mut graph = StateGraph { elements: vec![] }; add(tree, &mut graph);
        let binding = Binding::from_graph(&graph).unwrap();
        let gates = bind(&binding);
        assert_eq!(gates.iter().filter(|&&gate| gate).count(), 2);
        assert_eq!(gates.iter().filter(|&&gate| !gate).count(), 2);
        for (id, gate) in gates.iter().enumerate() {
            assert!(!blocked(false, &gates, id), "disabled trainer must preserve stock transitions");
            assert_eq!(blocked(true, &gates, id), *gate);
        }
        assert!(!blocked(true, &gates, usize::MAX));
    }
}
