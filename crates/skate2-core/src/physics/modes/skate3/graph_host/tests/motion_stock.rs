use super::*;

#[test]
#[ignore = "requires stock graph, collection and animation artifacts (not in the repository)"]
fn stock_motion_host_loads_graph_settings_and_authored_riding_tree() {
    use crate::skate3_native::graph_runtime::CompiledGraph;
    use skate_data::state_graph::{StateGraph, binding::Binding};
    let root = std::path::PathBuf::from(
        std::env::var_os("SKATE3_ASSET_ROOT").expect("Set SKATE3_ASSET_ROOT"),
    );
    let source =
        StateGraph::load(&root.join("private/stock/data/state/MotionGraph_OnBoard.stategraph"))
            .unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let runtime = CompiledGraph::from_binding(&binding).unwrap();
    let graph = LoadedGraph {
        source,
        binding,
        runtime,
    };
    let collections = Collections::load(&root).unwrap();
    let host = MotionHost::from_graph(
        &graph,
        &collections,
        skate_data::animation_banks::AnimationBanks::load(&root)
            .unwrap()
            .metadata()
            .unwrap(),
        PlaybackContext {
            is_switch: None,
            is_mirrored: None,
            board_available: None,
            pro_skater: encode(b""),
            transition_override: None,
        },
    )
    .unwrap();
    let tree = host.animation.build_tree("BTREE_RIDING").unwrap();
    fn count(tree: &skate3_core::animation::playback_tree::PlaybackTree) -> (usize, usize) {
        use skate3_core::animation::playback_tree::PlaybackTree;
        match tree {
            PlaybackTree::Clip { .. } => (1, 0),
            PlaybackTree::PhaseBlend(tree) => tree
                .children
                .iter()
                .map(count)
                .fold((0, 1), |(ac, ap), (bc, bp)| (ac + bc, ap + bp)),
            PlaybackTree::BlendSpace(tree) => tree.children.iter().map(count).fold((0,0), |(ac,ap),(bc,bp)| (ac+bc,ap+bp)),
            PlaybackTree::Transition(_) => {
                panic!("Authored bank unexpectedly contains a runtime transition")
            }
            PlaybackTree::BindPose { motion, .. } => count(motion),
            PlaybackTree::SelectionSpace(tree) => tree
                .candidates
                .iter()
                .map(|candidate| count(&candidate.tree))
                .fold((0, 0), |(ac, ap), (bc, bp)| (ac + bc, ap + bp)),
        }
    }
    let (clips, phases) = count(&tree);
    assert!(clips > 1 && phases > 1);
    for name in [
        "B_MONGO_PUSH_CYC1",
        "B_MONGO_PUSH_CYC2",
        "B_MONGO_PUSH_INTO",
        "B_PUSH_CYC1",
        "B_PUSH_CYC2",
        "B_PUSH_INTO",
        "B_PUSH_MONGO_OUT",
        "B_PUSH_OUT",
        "B_SWITCH",
    ] {
        let tree = host.animation.build_tree(name).unwrap();
        let (clips, phases) = count(&tree);
        assert!(
            clips > 0 && phases > 0,
            "Authored push root {name} lost its blend tree"
        );
    }
    println!(
        "Production host: {} operations, real riding closure: {clips} leaf instances, {phases} PhaseBlend instances",
        host.operations.len()
    );
}

/// Enumerate actual factory results; this is a support inventory, not a parity test.
#[test]
#[ignore = "requires the stock MotionGraph (not in the repository)"]
fn stock_motion_factory_support_inventory() {
    use skate_data::state_graph::{
        StateGraph,
        binding::{Binding, Node, OperationKind},
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("SKATE3_ASSET_ROOT").expect("Set SKATE3_ASSET_ROOT"),
    );
    let source =
        StateGraph::load(&root.join("private/stock/data/state/MotionGraph_OnBoard.stategraph"))
            .unwrap();
    let binding = Binding::from_graph(&source).unwrap();
    let instantiated = binding
        .instantiate_operations(&source, &mut MotionFactory)
        .unwrap();
    let mut rows = String::from(
        "kind\tname\tfactory_status\toperation_id\truntime_id\tenabled\towner\tvariant\n",
    );
    let mut counts = [0usize; 3];
    for (id, (operation, implementation)) in binding
        .operations
        .iter()
        .zip(&instantiated.operations)
        .enumerate()
    {
        let (kind, index) = match operation.kind {
            OperationKind::Behavior => ("Behavior", 0),
            OperationKind::Condition => ("Condition", 1),
            OperationKind::Hook => ("Hook", 2),
        };
        let status = if matches!(implementation, MotionOperation::Unsupported { .. }) {
            "Unsupported"
        } else {
            "Registered"
        };
        let owner = if let Node::State(state) = operation.parent {
            let mut names = Vec::new();
            let mut cursor = Some(state);
            while let Some(state) = cursor {
                names.push(binding.states[state].name.clone());
                cursor = binding.states[state].parent;
            }
            names.reverse();
            names.join(".")
        } else {
            format!("{:?}", operation.parent)
        };
        let clean = |s: String| s.replace(['\t', '\r', '\n'], " ");
        rows.push_str(&format!(
            "{kind}\t{}\t{status}\t{id}\t{}\t{}\t{}\t{}\n",
            clean(operation.name.clone()),
            counts[index],
            operation.enabled,
            clean(owner),
            clean(format!("{implementation:?}"))
        ));
        counts[index] += 1;
    }
    assert_eq!(counts.iter().sum::<usize>(), binding.operations.len());
    let output = std::path::PathBuf::from(
        std::env::var_os("SKATE3_MOTION_INVENTORY").expect("Set SKATE3_MOTION_INVENTORY"),
    );
    std::fs::write(&output, rows).unwrap();
    println!(
        "MotionGraph factory inventory: {} behaviors, {} conditions, {} hooks; {}",
        counts[0],
        counts[1],
        counts[2],
        output.display()
    );
}
