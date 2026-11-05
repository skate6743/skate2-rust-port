//! Native dynamic-controller selection: 82C140D8 and its complete state-search
//! helpers. Topology and flags come from loaded graph objects, never a riding
//! state machine. The activation callback is Node::GetActivation's boundary.

pub type StateId = usize;
pub type TransitionId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeId {
    State(StateId),
    Transition(TransitionId),
}

#[derive(Clone, Debug)]
pub struct State {
    pub parent: Option<StateId>,
    pub enabled: bool,
    pub active: bool,
    /// Native state+36: 0=false, 1=true, 2=resolved ancestor name.
    pub interruptibility: u32,
    pub interrupt_ancestor: Option<StateId>,
    pub children: Vec<StateId>,
    pub transitions: Vec<TransitionId>,
}

#[derive(Clone, Debug)]
pub struct Transition {
    pub enabled: bool,
    pub target: StateId,
    pub priority: u32,
}

#[derive(Clone, Debug)]
pub struct Topology {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

pub struct Selection<'a> {
    pub graph: &'a Topology,
    pub root: StateId,
    pub current: Option<StateId>,
}

impl Selection<'_> {
    /// Complete GetNextState 82C140D8. The output slot is explicit because an
    /// unrecognized interruptibility ordinal leaves it untouched in TU3.
    pub fn next(
        &self,
        target: &mut Option<StateId>,
        mut activate: impl FnMut(NodeId, u32) -> bool,
    ) -> Option<TransitionId> {
        let Some(current) = self.current else {
            *target = self.descend(self.root, 1, &mut activate);
            return None;
        };
        for priority in (0..=3).rev() {
            if let Some((transition, state)) =
                self.search_transitions(current, priority, &mut activate)
            {
                *target = Some(state);
                return Some(transition);
            }
        }
        // Complete retained/reselected-state path 82C14248.
        match self.graph.states[current].interruptibility {
            0 if self.ancestors_active(current, 2, &mut activate) => *target = Some(current),
            0 | 1 => *target = self.descend(self.root, 2, &mut activate),
            2 => {
                let ancestor = self.graph.states[current]
                    .interrupt_ancestor
                    .expect("native processed interrupt ancestor must resolve");
                if self.ancestors_active(ancestor, 2, &mut activate) {
                    *target = self.descend(ancestor, 2, &mut activate);
                    if target.is_some() {
                        return None;
                    }
                }
                *target = self.descend(self.root, 2, &mut activate);
            }
            _ => {}
        }
        None
    }

    /// Complete 82C14710 (82C14708 is its thunk).
    fn descend(
        &self,
        state: StateId,
        mask: u32,
        activate: &mut impl FnMut(NodeId, u32) -> bool,
    ) -> Option<StateId> {
        let effective_mask = if mask == 2 && self.is_ancestor(state, self.current) {
            2
        } else {
            1
        };
        if !activate(NodeId::State(state), effective_mask) {
            return None;
        }
        self.first_child_or_leaf(state, mask, activate)
    }

    fn first_child_or_leaf(
        &self,
        state: StateId,
        mask: u32,
        activate: &mut impl FnMut(NodeId, u32) -> bool,
    ) -> Option<StateId> {
        let children = &self.graph.states[state].children;
        if children.is_empty() {
            return Some(state);
        }
        for &child in children {
            let node = &self.graph.states[child];
            if node.enabled && node.active {
                if let Some(leaf) = self.descend(child, mask, activate) {
                    return Some(leaf);
                }
            }
        }
        None
    }

    /// Complete priority-specific SearchTransitions 82C14810, leaf to root.
    fn search_transitions(
        &self,
        mut state: StateId,
        priority: u32,
        activate: &mut impl FnMut(NodeId, u32) -> bool,
    ) -> Option<(TransitionId, StateId)> {
        loop {
            for &id in &self.graph.states[state].transitions {
                let transition = &self.graph.transitions[id];
                let target = transition.target;
                if transition.priority != priority
                    || !transition.enabled
                    || !self.graph.states[target].enabled
                {
                    continue;
                }
                if !activate(NodeId::Transition(id), 1)
                    || !self.valid_target(self.current, target, activate)
                    || !activate(NodeId::State(target), 1)
                {
                    continue;
                }
                if let Some(leaf) = self.first_child_or_leaf(target, 1, activate) {
                    return Some((id, leaf));
                }
            }
            state = self.graph.states[state].parent?;
        }
    }

    /// Complete ValidTarget 82C14978. Target and new ancestors use preconditions;
    /// a shared ancestor (other than target) and its parents use mask 2.
    fn valid_target(
        &self,
        current: Option<StateId>,
        target: StateId,
        activate: &mut impl FnMut(NodeId, u32) -> bool,
    ) -> bool {
        let mut next = Some(target);
        while let Some(state) = next {
            if state != target && self.is_ancestor(state, current) {
                return self.ancestors_active(state, 2, activate);
            }
            if !activate(NodeId::State(state), 1) {
                return false;
            }
            next = self.graph.states[state].parent;
        }
        true
    }

    /// Complete state-parent activation chain 82C16B88 for State objects.
    fn ancestors_active(
        &self,
        mut state: StateId,
        mask: u32,
        activate: &mut impl FnMut(NodeId, u32) -> bool,
    ) -> bool {
        loop {
            if !activate(NodeId::State(state), mask) {
                return false;
            }
            let Some(parent) = self.graph.states[state].parent else {
                return true;
            };
            state = parent;
        }
    }

    fn is_ancestor(&self, ancestor: StateId, mut state: Option<StateId>) -> bool {
        while let Some(id) = state {
            if id == ancestor {
                return true;
            }
            state = self.graph.states[id].parent;
        }
        false
    }
}
