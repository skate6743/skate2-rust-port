use super::*;
use skate3_core::animation::{
    output::Sqt,
    playback_tree::{Evaluation, PlaybackTree},
};

fn finite(pose: &[Sqt]) {
    assert!(!pose.is_empty());
    for (i, bone) in pose.iter().enumerate() {
        assert!(
            bone.scale
                .iter()
                .chain(&bone.rotation)
                .chain(&bone.translation)
                .all(|v| v.is_finite()),
            "nonfinite bump bone {i}"
        );
    }
}

#[test]
#[ignore = "requires stock animation and graph assets (not in the repository)"]
fn stock_bump_crash_regression() {
    use crate::skate3_native::{animation_pose::PoseEvaluator, graph_runtime::CompiledGraph};
    use skate_data::{
        animation_banks::AnimationBanks,
        state_graph::{StateGraph, binding::Binding},
    };
    let root = std::path::PathBuf::from(
        std::env::var_os("SKATE3_ASSET_ROOT").expect("Set SKATE3_ASSET_ROOT"),
    );
    let banks = AnimationBanks::load(&root).unwrap();
    let evaluator = PoseEvaluator::from_banks(&banks).unwrap();
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
    let mut host = MotionHost::from_graph(
        &graph,
        &collections,
        banks.metadata().unwrap(),
        PlaybackContext {
            is_switch: Some(false),
            is_mirrored: Some(false),
            board_available: Some(true),
            pro_skater: encode(b""),
            transition_override: None,
        },
    )
    .unwrap();
    host.animation
        .set_hierarchy(
            &evaluator.frames.bone_names,
            &evaluator.frames.mirror_indices,
        )
        .unwrap();
    let mut tree = host.animation.build_tree("B_BUMP").unwrap();
    let PlaybackTree::BlendSpace(space) = &tree else {
        panic!("B_BUMP lost its type6 topology")
    };
    assert_eq!(
        space.parameters,
        vec![encode(b"DISTTOCOG"), encode(b"BUMPX"), encode(b"BUMPY")]
    );
    assert_eq!(space.children.len(), 26);
    assert_eq!(space.simplexes.len(), 48);
    let parameters = space.parameters.clone();
    let points = space
        .simplexes
        .iter()
        .flat_map(|s| {
            let center = (0..3)
                .map(|j| s.vertices.iter().map(|v| v[j]).sum::<f32>() / 4.)
                .collect();
            s.vertices
                .iter()
                .cloned()
                .chain(std::iter::once(center))
                .collect::<Vec<_>>()
        })
        .chain([vec![0., 0., 0.], vec![2., 2., -2.], vec![0.8, 0., 0.]])
        .collect::<Vec<_>>();
    let evaluation = Evaluation {
        cull_threshold: 0.01,
        update_history: true,
    };
    //Decode every source clip and evaluate authored vertices/interiors. No physics or gameplay automation.
    for child in &space.children {
        let mut child = child.clone();
        child.set_time(child.length() * 0.4);
        let mut commands = Vec::new();
        child.evaluate(evaluation, true, &mut commands).unwrap();
        finite(&evaluator.evaluate(&commands).unwrap());
    }
    for point in &points {
        tree.set_attributes(
            &parameters
                .iter()
                .zip(point)
                .map(|(&name, &value)| SettableAttribute {
                    name,
                    value,
                    normalized: false,
                    sequence_id: -1,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        tree.set_time(tree.length() * 0.4);
        let mut commands = Vec::new();
        tree.evaluate(evaluation, true, &mut commands).unwrap();
        let pose = evaluator.evaluate(&commands).unwrap();
        finite(&pose);
        for bone in &pose {
            let norm: f32 = bone.rotation.iter().map(|v| v * v).sum();
            assert!((norm - 1.).abs() < 1e-5, "weighted quaternion norm {norm}");
        }
        tree.attributes(15).unwrap();
        let PlaybackTree::BlendSpace(space) = &tree else {
            unreachable!()
        };
        let weights = space.active_weights().map(|(_, w)| w).collect::<Vec<_>>();
        assert!(weights.iter().all(|&w| w >= 0. && w <= 1.));
        if points[..240].iter().any(|p| p == point) {
            let simplex = space
                .simplexes
                .iter()
                .find(|s| {
                    s.children
                        .iter()
                        .copied()
                        .eq(space.active_weights().map(|(i, _)| i))
                })
                .unwrap();
            for j in 0..3 {
                let actual = simplex
                    .vertices
                    .iter()
                    .zip(&weights)
                    .map(|(v, w)| v[j] * w)
                    .sum::<f32>();
                assert!(
                    (actual - point[j]).abs() < 2e-5,
                    "authored point {point:?} reconstructed as {actual} on axis {j}"
                );
            }
        }
        assert!((weights.iter().sum::<f32>() - 1.).abs() < 1e-6);
    }
    //Exact runtime behavior IDs from a crash log.
    assert!(
        matches!(&host.operations[host.remap.behaviors[3623]], MotionOperation::Play(p) if p.animation.eq_ignore_ascii_case("B_BUMP"))
    );
    assert!(
        matches!(&host.operations[host.remap.behaviors[3624]], MotionOperation::SetBumpCoefficients(names) if *names==[encode(b"BUMPX"),encode(b"BUMPY")])
    );
    let frame = Frame {
        dt: 1. / 60.,
        current: None,
        last: None,
        state_times: Vec::new(),
    };
    for mirrored in [false, true] {
        host.animation.reset_from_stock();
        host.animation.skater_animation_flags = Some(if mirrored { 0x4000_0000 } else { 0 });
        host.bump_acceleration = Some([150., 0., -150., 0.]);
        host.execute(3623, &frame, 0).unwrap();
        host.execute(3624, &frame, 0).unwrap();
        host.animation.apply_parameters().unwrap();
        host.animation.advance(1. / 60., 0.);
        host.animation.refresh_tree_attributes().unwrap();
        finite(
            &evaluator
                .evaluate(&host.animation.evaluate_pose(evaluation).unwrap())
                .unwrap(),
        );
        //Update/End must not recapture the impact.
        host.bump_acceleration = None;
        host.execute(3624, &frame, 1).unwrap();
        host.execute(3624, &frame, 2).unwrap();
    }
    println!(
        "Bump regression: 26 stock clips, 48 simplexes, {} parameter probes, logged graph behaviors 3623/3624 in both stances",
        points.len()
    );
}
