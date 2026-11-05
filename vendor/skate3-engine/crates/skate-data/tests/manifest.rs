use skate_data::GameAssets;

const MOTION_GRAPH: &str = "private/stock/data/state/MotionGraph_OnBoard.stategraph";

fn manifest(motion_graph: &str, version: u32) -> String {
    serde_json::json!({
        "version": version,
        "action_graph": "private/stock/data/state/ActionGraph_OnBoard.stategraph",
        "motion_graph": motion_graph
    })
    .to_string()
}
#[test]
fn validates_contract_without_loading_private_assets() {
    assert_eq!(
        GameAssets::parse(&manifest(MOTION_GRAPH, 1))
            .unwrap()
            .motion_graph,
        MOTION_GRAPH
    );
}
#[test]
fn rejects_incompatible_versions_and_ambiguous_paths() {
    assert!(GameAssets::parse(&manifest(MOTION_GRAPH, 2)).is_err());
    for path in [
        "../MotionGraph_OnBoard.stategraph",
        "/MotionGraph_OnBoard.stategraph",
        "C:/MotionGraph_OnBoard.stategraph",
        "private\\MotionGraph_OnBoard.stategraph",
        "private/MotionGraph_OnBoard.stategraph#Scene0",
        "private/MotionGraph_OnBoard.json",
        "",
    ] {
        assert!(
            GameAssets::parse(&manifest(path, 1)).is_err(),
            "accepted {path}"
        );
    }
}
