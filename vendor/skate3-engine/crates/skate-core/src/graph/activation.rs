//! Data-driven activation for the stock state graph.
//!
//! Node::GetActivation 82C14C28 delegates to its expression with the caller's
//! mask. Condition::GetActivationMasked 82C12D48 excludes disabled conditions
//! and conditions whose authored mask does not intersect that mask. Expression
//! combination and its excluded-child rules are implemented in `expression`.

use super::{controller::Frame, expression::evaluate_operator, selection::NodeId};

pub type ConditionId = usize;
pub type ExpressionId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Child {
    Expression(ExpressionId),
    Condition(ConditionId),
}

#[derive(Clone, Debug)]
pub struct Expression {
    pub operator: u32,
    /// Native child vector order. Evaluation is lazy, so this order is visible.
    pub children: Vec<Child>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Condition {
    pub enabled: bool,
    /// Parsed CondMask bits from the stock condition's `mask` attribute.
    pub mask: u32,
}

#[derive(Clone, Debug)]
pub struct ActivationProgram {
    pub state_expressions: Vec<Option<ExpressionId>>,
    pub transition_expressions: Vec<Option<ExpressionId>>,
    pub expressions: Vec<Expression>,
    pub conditions: Vec<Condition>,
}

/// Concrete condition implementations are registered by the game. The graph
/// runtime decides whether they run; the host only evaluates an included leaf.
pub trait ConditionHost {
    fn condition_activation(&mut self, condition: ConditionId, frame: &Frame) -> u32;
}

impl ActivationProgram {
    /// Complete Node::GetActivation boundary (82C14C28).
    pub fn node_activation(
        &self,
        node: NodeId,
        mask: u32,
        frame: &Frame,
        host: &mut impl ConditionHost,
    ) -> bool {
        let expression = match node {
            NodeId::State(id) => self.state_expressions[id],
            NodeId::Transition(id) => self.transition_expressions[id],
        };
        let Some(expression) = expression else {
            return true;
        };
        let mut excluded = 1;
        let result = self.expression_activation(expression, mask, frame, &mut excluded, host);
        excluded != 0 || result as u8 != 0
    }

    fn expression_activation(
        &self,
        id: ExpressionId,
        mask: u32,
        frame: &Frame,
        excluded: &mut u8,
        host: &mut impl ConditionHost,
    ) -> u32 {
        let expression = &self.expressions[id];
        evaluate_operator(
            expression.operator,
            expression.children.len(),
            excluded,
            |index, child_excluded| match expression.children[index] {
                Child::Expression(child) => {
                    self.expression_activation(child, mask, frame, child_excluded, host)
                }
                Child::Condition(child) => {
                    self.condition_activation(child, mask, frame, child_excluded, host)
                }
            },
        )
    }

    /// Complete Condition::GetActivationMasked gate (82C12D48).
    fn condition_activation(
        &self,
        id: ConditionId,
        mask: u32,
        frame: &Frame,
        excluded: &mut u8,
        host: &mut impl ConditionHost,
    ) -> u32 {
        let condition = self.conditions[id];
        if !condition.enabled || condition.mask & mask == 0 {
            *excluded = 1;
            1
        } else {
            *excluded = 0;
            host.condition_activation(id, frame)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Host {
        values: Vec<u32>,
        calls: Vec<ConditionId>,
    }

    impl ConditionHost for Host {
        fn condition_activation(&mut self, condition: ConditionId, _: &Frame) -> u32 {
            self.calls.push(condition);
            self.values[condition]
        }
    }

    fn frame() -> Frame {
        Frame {
            dt: 0.0,
            current: None,
            last: None,
            state_times: Vec::new(),
        }
    }

    fn program(operator: u32, conditions: Vec<Condition>) -> ActivationProgram {
        ActivationProgram {
            state_expressions: vec![Some(0)],
            transition_expressions: Vec::new(),
            expressions: vec![Expression {
                operator,
                children: (0..conditions.len()).map(Child::Condition).collect(),
            }],
            conditions,
        }
    }

    #[test]
    fn excluded_conditions_do_not_participate_and_all_excluded_is_true() {
        let graph = program(
            1,
            vec![
                Condition {
                    enabled: false,
                    mask: 1,
                },
                Condition {
                    enabled: true,
                    mask: 2,
                },
            ],
        );
        let mut host = Host {
            values: vec![0, 0],
            ..Default::default()
        };
        assert!(graph.node_activation(NodeId::State(0), 1, &frame(), &mut host));
        assert!(host.calls.is_empty());
    }

    #[test]
    fn included_false_condition_controls_result_and_or_short_circuits() {
        let graph = program(
            2,
            vec![
                Condition {
                    enabled: true,
                    mask: 1,
                },
                Condition {
                    enabled: true,
                    mask: 1,
                },
            ],
        );
        let mut host = Host {
            values: vec![1, 0],
            ..Default::default()
        };
        assert!(graph.node_activation(NodeId::State(0), 1, &frame(), &mut host));
        assert_eq!(host.calls, [0]);

        host.values[0] = 0;
        host.calls.clear();
        assert!(!graph.node_activation(NodeId::State(0), 1, &frame(), &mut host));
        assert_eq!(host.calls, [0, 1]);
    }

    #[test]
    fn nodes_without_expressions_are_active() {
        let graph = ActivationProgram {
            state_expressions: vec![None],
            transition_expressions: vec![None],
            expressions: Vec::new(),
            conditions: Vec::new(),
        };
        let mut host = Host::default();
        assert!(graph.node_activation(NodeId::State(0), 1, &frame(), &mut host));
        assert!(graph.node_activation(NodeId::Transition(0), 2, &frame(), &mut host));
    }
}
