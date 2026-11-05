//! Lossless bridge from the stock on-disk graph binding to the recovered TU3
//! dynamic-controller runtime.
//!
//! Concrete operations are intentionally retained by source operation ID. The
//! registration layer must construct every named operation before either graph
//! can execute; this module never substitutes a default handler.


use skate3_core::graph::{
    activation::{ActivationProgram, Child, Condition, Expression},
    controller::{Behavior, Program},
    selection::{State, Topology, Transition},
};
use skate_data::{
    GameAssets,
    state_graph::{
        StateGraph,
        binding::{Binding, Node, OperationKind},
    },
};
use std::{fmt, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GraphCompileError {
    UnresolvedTransitionTarget {
        transition: usize,
    },
    InvalidOperationParent {
        operation: usize,
        expected: &'static str,
    },
    InvalidStateBehavior {
        state: usize,
        operation: usize,
    },
    InvalidTransitionHook {
        transition: usize,
        operation: usize,
    },
    InvalidExpressionChild {
        expression: usize,
        node: Node,
    },
    MissingConditionMask {
        operation: usize,
    },
}

impl fmt::Display for GraphCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedTransitionTarget { transition } => {
                write!(
                    f,
                    "stock graph transition {transition} has no resolved target"
                )
            }
            Self::InvalidOperationParent {
                operation,
                expected,
            } => write!(
                f,
                "stock graph operation {operation} does not have its required {expected} parent"
            ),
            Self::InvalidStateBehavior { state, operation } => write!(
                f,
                "stock graph state {state} references non-behavior operation {operation}"
            ),
            Self::InvalidTransitionHook {
                transition,
                operation,
            } => write!(
                f,
                "stock graph transition {transition} references non-hook operation {operation}"
            ),
            Self::InvalidExpressionChild { expression, node } => write!(
                f,
                "stock graph expression {expression} has invalid child {node:?}"
            ),
            Self::MissingConditionMask { operation } => {
                write!(
                    f,
                    "stock graph condition {operation} has no parsed TU3 mask"
                )
            }
        }
    }
}

impl std::error::Error for GraphCompileError {}

/// Compact runtime IDs map back to the original binding operation IDs. The
/// future concrete host uses these tables to reach its exact factory instances.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperationRemap {
    pub behaviors: Vec<usize>,
    pub conditions: Vec<usize>,
    pub hooks: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledGraph {
    pub program: Program,
    pub operations: OperationRemap,
}

impl CompiledGraph {
    pub fn from_binding(binding: &Binding) -> Result<Self, GraphCompileError> {
        let operation_count = binding.operations.len();
        let mut behavior_for_operation = vec![None; operation_count];
        let mut condition_for_operation = vec![None; operation_count];
        let mut hook_for_operation = vec![None; operation_count];
        let mut behavior_operations = Vec::new();
        let mut condition_operations = Vec::new();
        let mut hook_operations = Vec::new();

        for (operation_id, operation) in binding.operations.iter().enumerate() {
            match operation.kind {
                OperationKind::Behavior => {
                    behavior_for_operation[operation_id] = Some(behavior_operations.len());
                    behavior_operations.push(operation_id);
                }
                OperationKind::Condition => {
                    condition_for_operation[operation_id] = Some(condition_operations.len());
                    condition_operations.push(operation_id);
                }
                OperationKind::Hook => {
                    hook_for_operation[operation_id] = Some(hook_operations.len());
                    hook_operations.push(operation_id);
                }
            }
        }

        let topology = Topology {
            states: binding
                .states
                .iter()
                .map(|state| State {
                    parent: state.parent,
                    enabled: state.enabled != 0,
                    active: state.active != 0,
                    interruptibility: state.interruptibility,
                    interrupt_ancestor: state.interrupt_ancestor,
                    children: state.children.clone(),
                    transitions: state.transitions.clone(),
                })
                .collect(),
            transitions: binding
                .transitions
                .iter()
                .enumerate()
                .map(|(id, transition)| {
                    Ok(Transition {
                        enabled: transition.enabled != 0,
                        target: transition.target.ok_or(
                            GraphCompileError::UnresolvedTransitionTarget { transition: id },
                        )?,
                        priority: transition.priority,
                    })
                })
                .collect::<Result<_, GraphCompileError>>()?,
        };

        let conditions = condition_operations
            .iter()
            .map(|&operation_id| {
                let operation = &binding.operations[operation_id];
                Ok(Condition {
                    enabled: operation.enabled != 0,
                    mask: operation.condition_mask.ok_or(
                        GraphCompileError::MissingConditionMask {
                            operation: operation_id,
                        },
                    )?,
                })
            })
            .collect::<Result<_, GraphCompileError>>()?;

        let expressions = binding
            .expressions
            .iter()
            .enumerate()
            .map(|(expression_id, expression)| {
                let children = expression
                    .children
                    .iter()
                    .map(|node| match *node {
                        Node::Expression(id) => Ok(Child::Expression(id)),
                        Node::Operation(operation) => condition_for_operation[operation]
                            .map(Child::Condition)
                            .ok_or(GraphCompileError::InvalidExpressionChild {
                                expression: expression_id,
                                node: *node,
                            }),
                        _ => Err(GraphCompileError::InvalidExpressionChild {
                            expression: expression_id,
                            node: *node,
                        }),
                    })
                    .collect::<Result<_, GraphCompileError>>()?;
                Ok(Expression {
                    operator: expression.operator,
                    children,
                })
            })
            .collect::<Result<_, GraphCompileError>>()?;

        let behaviors = behavior_operations
            .iter()
            .map(|&operation_id| {
                let operation = &binding.operations[operation_id];
                let Node::State(owner) = operation.parent else {
                    return Err(GraphCompileError::InvalidOperationParent {
                        operation: operation_id,
                        expected: "state",
                    });
                };
                Ok(Behavior {
                    owner,
                    enabled: operation.enabled != 0,
                })
            })
            .collect::<Result<_, GraphCompileError>>()?;

        let state_behaviors = binding
            .states
            .iter()
            .enumerate()
            .map(|(state, source)| {
                source
                    .behaviors
                    .iter()
                    .map(|&operation| {
                        behavior_for_operation[operation]
                            .ok_or(GraphCompileError::InvalidStateBehavior { state, operation })
                    })
                    .collect()
            })
            .collect::<Result<_, GraphCompileError>>()?;

        let transition_hooks = binding
            .transitions
            .iter()
            .enumerate()
            .map(|(transition, source)| {
                source
                    .hooks
                    .iter()
                    .map(|&operation| {
                        hook_for_operation[operation].ok_or(
                            GraphCompileError::InvalidTransitionHook {
                                transition,
                                operation,
                            },
                        )
                    })
                    .collect()
            })
            .collect::<Result<_, GraphCompileError>>()?;

        Ok(Self {
            program: Program {
                topology,
                root: binding.root,
                activation: ActivationProgram {
                    state_expressions: binding
                        .states
                        .iter()
                        .map(|state| state.expression)
                        .collect(),
                    transition_expressions: binding
                        .transitions
                        .iter()
                        .map(|transition| transition.expression)
                        .collect(),
                    expressions,
                    conditions,
                },
                behaviors,
                state_behaviors,
                transition_hooks,
            },
            operations: OperationRemap {
                behaviors: behavior_operations,
                conditions: condition_operations,
                hooks: hook_operations,
            },
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LoadedGraph {
    pub source: StateGraph,
    pub binding: Binding,
    pub runtime: CompiledGraph,
}

#[derive(Clone, Debug)]
pub(crate) struct StockGraphs {
    pub action: LoadedGraph,
    pub motion: LoadedGraph,
}

impl StockGraphs {
    pub fn load(root: &Path, manifest: &GameAssets) -> Result<Self, String> {
        Ok(Self {
            action: load_graph(root, &manifest.action_graph)?,
            motion: load_graph(root, &manifest.motion_graph)?,
        })
    }
}

fn load_graph(root: &Path, relative: &str) -> Result<LoadedGraph, String> {
    let path = root.join(relative);
    let source = StateGraph::load(&path).map_err(|error| error.to_string())?;
    let binding = Binding::from_graph(&source).map_err(|error| error.to_string())?;
    let runtime = CompiledGraph::from_binding(&binding)
        .map_err(|error| format!("Invalid runtime graph {}: {error}", path.display()))?;
    Ok(LoadedGraph {
        source,
        binding,
        runtime,
    })
}

#[cfg(any())]
#[path = "tests/graph_runtime.rs"]
mod tests;
